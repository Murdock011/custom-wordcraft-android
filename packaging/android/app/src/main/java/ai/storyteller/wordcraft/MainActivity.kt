package ai.storyteller.wordcraft

import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.util.Base64
import android.util.Log
import android.webkit.JavascriptInterface
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebChromeClient.FileChooserParams
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.webkit.WebViewAssetLoader
import androidx.webkit.WebViewCompat
import org.json.JSONObject
import java.io.IOException
import java.util.concurrent.ConcurrentHashMap

/**
 * Shows the browser build of WordCraft (`apps/wordcraft-web`) full screen. The page is the phone
 * layout: `index.html` is served from the app's own assets, so it runs from a secure origin with
 * the WASM module fetched from the same place.
 */
class MainActivity : AppCompatActivity() {
    private var pendingFiles: ValueCallback<Array<Uri>>? = null

    /** A save waiting for the user to pick where the file goes (Save As, or a first Save). */
    private class PendingSave(val name: String, val mime: String, val bytes: ByteArray)
    private var waitingSave: PendingSave? = null

    /** Where each document was last saved, by the name the page gives it. Kept for this session. */
    private val savedTo = ConcurrentHashMap<String, Uri>()

    private var web: WebView? = null

    private val createDocument = registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
        val pending = waitingSave ?: return@registerForActivityResult
        waitingSave = null
        val uri = result.data?.data
        if (result.resultCode != RESULT_OK || uri == null) {
            Log.i(TAG, "save of ${pending.name} cancelled at the picker")
            reportSave(pending.name, "cancelled", "")
            return@registerForActivityResult
        }
        try {
            writeTo(uri, pending.bytes)
            // Keep access to the file across app restarts, if the provider allows it.
            try {
                contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
            } catch (e: SecurityException) {
                Log.w(TAG, "no persistable permission for $uri", e)
            }
            savedTo[pending.name] = uri
            Log.i(TAG, "saved ${pending.name} to $uri")
            reportSave(pending.name, "saved", "")
        } catch (e: Exception) {
            Log.e(TAG, "saving ${pending.name} to $uri failed", e)
            reportSave(pending.name, "failed", describe(e))
        }
    }

    private val picker = registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
        val data = result.data
        val uris: Array<Uri>? = if (result.resultCode == RESULT_OK && data != null) {
            data.data?.let { arrayOf(it) } ?: data.clipData?.let { clip ->
                Array(clip.itemCount) { clip.getItemAt(it).uri }
            }
        } else {
            null
        }
        pendingFiles?.onReceiveValue(uris)
        pendingFiles = null
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val loader = WebViewAssetLoader.Builder()
            .addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(this))
            .build()
        val web = WebView(this)
        // Android 15 draws edge to edge whatever the window asks for, so the page is padded away
        // from the status bar, the navigation bar and the keyboard. The padding sits on a container
        // around the WebView, because the WebView keeps the insets to itself.
        val container = FrameLayout(this).apply { addView(web) }
        setContentView(container)
        ViewCompat.setOnApplyWindowInsetsListener(container) { view, insets ->
            val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
            val keyboard = insets.getInsets(WindowInsetsCompat.Type.ime())
            view.setPadding(bars.left, bars.top, bars.right, maxOf(bars.bottom, keyboard.bottom))
            WindowInsetsCompat.CONSUMED
        }
        window.statusBarColor = BAR_COLOR
        window.navigationBarColor = BAR_COLOR
        WindowInsetsControllerCompat(window, container).isAppearanceLightStatusBars = true
        web.isFocusable = true
        web.isFocusableInTouchMode = true
        web.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            cacheMode = WebSettings.LOAD_DEFAULT
        }
        // Saves: the page downloads a file through a blob link, which a WebView can't download by
        // itself. The script below hands those bytes to SaveBridge, which reports back to the page.
        this.web = web
        web.addJavascriptInterface(SaveBridge(this), "AndroidSave")
        // Back closes what the page has open (a sheet page, the sheet, a menu, Backstage) first:
        // the page says so in window.wcCanGoBack, and Escape closes one level. Otherwise Back leaves.
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                web.evaluateJavascript("window.wcCanGoBack === true") { result ->
                    if (result == "true") {
                        web.evaluateJavascript(
                            "document.getElementById('wordcraft_canvas').dispatchEvent(new KeyboardEvent('keydown', {key: 'Escape', code: 'Escape', bubbles: true}))",
                            null,
                        )
                    } else {
                        isEnabled = false
                        onBackPressedDispatcher.onBackPressed()
                        isEnabled = true
                    }
                }
            }
        })
        WebViewCompat.addDocumentStartJavaScript(web, SAVE_SHIM, setOf(ORIGIN))
        web.webViewClient = object : WebViewClient() {
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? =
                loader.shouldInterceptRequest(request.url)
        }
        // The web build opens files through an <input type="file">: hand that to the system picker.
        web.webChromeClient = object : WebChromeClient() {
            override fun onShowFileChooser(
                view: WebView,
                callback: ValueCallback<Array<Uri>>,
                params: FileChooserParams,
            ): Boolean {
                pendingFiles?.onReceiveValue(null)
                pendingFiles = callback
                picker.launch(params.createIntent())
                return true
            }
        }
        web.loadUrl("$ORIGIN/assets/web/index.html")
        // The page takes the keyboard only while its window has focus: give the WebView it.
        web.requestFocus()
    }

    /** Saves a file the page is handing over. Calls run on the WebView's thread. */
    private class SaveBridge(private val activity: MainActivity) {
        @JavascriptInterface
        fun save(name: String, base64: String, mime: String, saveAs: Boolean) {
            activity.save(name, base64, mime, saveAs)
        }
    }

    /**
     * Save: writes to the file this document was last saved to (picked in this session). Save As,
     * a first save, or a write that fails: opens the system Save picker (Storage Access Framework)
     * so the user chooses the name and place. The page is told the outcome through wcSaveResult.
     */
    private fun save(name: String, base64: String, mime: String, saveAs: Boolean) {
        val bytes = try {
            Base64.decode(base64, Base64.DEFAULT)
        } catch (e: IllegalArgumentException) {
            Log.e(TAG, "save of $name: bad data", e)
            reportSave(name, "failed", describe(e))
            return
        }
        val known = savedTo[name]
        if (!saveAs && known != null) {
            try {
                writeTo(known, bytes)
                Log.i(TAG, "saved $name to $known")
                reportSave(name, "saved", "")
                return
            } catch (e: Exception) {
                Log.e(TAG, "saving $name to $known failed; asking where to save", e)
            }
        }
        runOnUiThread {
            waitingSave?.let { reportSave(it.name, "cancelled", "") }
            waitingSave = PendingSave(name, mime, bytes)
            try {
                createDocument.launch(
                    Intent(Intent.ACTION_CREATE_DOCUMENT)
                        .addCategory(Intent.CATEGORY_OPENABLE)
                        .setType(mime)
                        .putExtra(Intent.EXTRA_TITLE, name),
                )
            } catch (e: Exception) {
                Log.e(TAG, "no save picker for $name", e)
                waitingSave = null
                reportSave(name, "failed", describe(e))
            }
        }
    }

    private fun writeTo(uri: Uri, bytes: ByteArray) {
        val out = contentResolver.openOutputStream(uri, "wt") ?: throw IOException("no output stream for $uri")
        out.use { it.write(bytes) }
    }

    /** Tells the page how a save ended: "saved", "cancelled" or "failed" (with the error text). */
    private fun reportSave(name: String, status: String, error: String) {
        runOnUiThread {
            if (status == "saved") Toast.makeText(this, "Saved $name to Downloads or the chosen folder", Toast.LENGTH_SHORT).show()
            web?.evaluateJavascript(
                "window.wcSaveResult && window.wcSaveResult(${JSONObject.quote(name)}, ${JSONObject.quote(status)}, ${JSONObject.quote(error)})",
                null,
            )
        }
    }

    private fun describe(e: Exception): String = "${e.javaClass.simpleName}: ${e.message}"

    private companion object {
        const val ORIGIN = "https://appassets.androidplatform.net"
        const val TAG = "WordCraftSave"

        // The bar colour of the phone layout (`title_bar` in theme.rs, light theme).
        val BAR_COLOR: Int = Color.rgb(240, 240, 240)

        // A click on a blob download link (how the web build saves) goes to AndroidSave.save
        // instead; every other click runs as usual.
        val SAVE_SHIM = """
            (function () {
              const nativeClick = HTMLAnchorElement.prototype.click;
              HTMLAnchorElement.prototype.click = function () {
                const href = this.href || '';
                if (this.hasAttribute('download') && href.startsWith('blob:')) {
                  const name = this.getAttribute('download') || 'WordCraft';
                  const saveAs = this.getAttribute('data-save-as') === 'true';
                  fetch(href).then(function (r) { return r.blob(); }).then(function (blob) {
                    const reader = new FileReader();
                    reader.onload = function () {
                      const base64 = String(reader.result).split(',')[1] || '';
                      AndroidSave.save(name, base64, blob.type || 'application/octet-stream', saveAs);
                    };
                    reader.readAsDataURL(blob);
                  }).catch(function (e) {
                    if (window.wcSaveResult) window.wcSaveResult(name, 'failed', String(e));
                  });
                  return;
                }
                return nativeClick.call(this);
              };
            })();
        """.trimIndent()
    }
}
