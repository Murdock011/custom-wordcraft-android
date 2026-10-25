package ai.storyteller.wordcraft.nativeapp

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.result.contract.ActivityResultContract
import java.io.File
import android.os.Bundle
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.print.PageRange
import android.print.PrintAttributes
import android.print.PrintDocumentAdapter
import android.print.PrintDocumentInfo
import android.print.PrintManager
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import android.widget.FrameLayout
import androidx.activity.OnBackPressedCallback
import android.graphics.Color
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import com.google.androidgamesdk.GameActivity

/**
 * GameActivity with the system-bar and keyboard insets applied as padding on the content view, so
 * the native surface is laid out above the status bar, the navigation bar and the soft keyboard.
 *
 * Text input goes through [ImeView], a 1x1 focusable view with its own InputConnection, so the
 * keyboard composes and commits text the way it does in a text field. GameActivity's own
 * GameTextInput is not used for typing.
 */
class MainActivity : GameActivity() {
    private val handler = Handler(Looper.getMainLooper())
    private var keyboardShown = false
    private lateinit var imeView: ImeView

    private val keyboardPoll = object : Runnable {
        override fun run() {
            nativeTakeRequest().takeIf { it.isNotEmpty() }?.let { files.add(it); nextFile() }
            nativeTakeClip().takeIf { it.isNotEmpty() }?.let { clipboard.setPrimaryClip(ClipData.newPlainText("WordCraft", it.substring(1))) }
            val want = nativeWantsKeyboard()
            if (want != keyboardShown) {
                keyboardShown = want
                if (want) showKeyboard() else hideKeyboard()
            } else if (want && !imeView.isFocused) {
                // A tap on the game surface takes focus (and the keyboard connection) from ImeView.
                showKeyboard()
            }
            handler.postDelayed(this, 150)
        }
    }

    // File requests from Rust (files.rs): "save\tticket\tname\tpath", "write\tticket\turi\tpath",
    // "open\tpurpose" or "print\tpath", one picker at a time.
    private val files = ArrayDeque<String>()
    private var filesBusy = false
    private var saveTicket = 0L
    private var saveSource = ""

    private class CreateDoc : ActivityResultContract<Pair<String, String>, Uri?>() {
        override fun createIntent(context: Context, input: Pair<String, String>) =
            Intent(Intent.ACTION_CREATE_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType(input.second).putExtra(Intent.EXTRA_TITLE, input.first)
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)

        override fun parseResult(resultCode: Int, intent: Intent?): Uri? = if (resultCode == RESULT_OK) intent?.data else null
    }

    private class OpenDoc : ActivityResultContract<Array<String>, Uri?>() {
        override fun createIntent(context: Context, input: Array<String>) =
            Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("*/*").putExtra(Intent.EXTRA_MIME_TYPES, input)
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)

        override fun parseResult(resultCode: Int, intent: Intent?): Uri? = if (resultCode == RESULT_OK) intent?.data else null
    }

    private val createDoc = registerForActivityResult(CreateDoc()) { uri ->
        val ticket = saveTicket
        val src = File(saveSource)
        try {
            if (uri == null) {
                nativeSaveDone(ticket, 0, "", "")
            } else {
                contentResolver.openOutputStream(uri, "wt")!!.use { out -> src.inputStream().use { it.copyTo(out) } }
                keepAccess(uri)
                nativeSaveDone(ticket, 1, uri.toString(), displayName(uri))
            }
        } catch (e: Exception) {
            nativeSaveDone(ticket, -1, e.message ?: e.toString(), "")
        }
        src.delete()
        filesBusy = false
        nextFile()
    }

    private val openDoc = registerForActivityResult(OpenDoc()) { uri ->
        if (uri != null) {
            try {
                val name = displayName(uri)
                val dest = File(cacheDir, "open-${System.nanoTime()}.bin")
                contentResolver.openInputStream(uri)!!.use { input -> dest.outputStream().use { input.copyTo(it) } }
                keepAccess(uri)
                nativeOpened(name, dest.path, uri.toString())
            } catch (e: Exception) {
                android.util.Log.w("wordcraft", "open failed: $e")
            }
        }
        filesBusy = false
        nextFile()
    }

    /** Keep read and write access to a picked document after a restart, if the provider allows it. */
    private fun keepAccess(uri: Uri) {
        try {
            contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        } catch (e: SecurityException) {
            // Read only, or not persistable: keep the read access if the provider allows it. Otherwise
            // the picker's access lasts while the app runs, and Recent says so after a restart.
            try {
                contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
            } catch (e2: SecurityException) {
                // Not persistable at all.
            }
        }
    }

    /** The provider's display name for [uri], or its last path segment when the provider has none. */
    private fun displayName(uri: Uri): String {
        val name = try {
            contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
                if (c.moveToFirst()) c.getString(0) else null
            }
        } catch (e: Exception) {
            null
        }
        return name ?: uri.lastPathSegment ?: "document"
    }

    private fun nextFile() {
        if (filesBusy) return
        val line = files.removeFirstOrNull() ?: return
        val f = line.split('\t')
        if (f[0] == "write") {
            // Plain Save of a document picked before: rewrite it without a picker. On failure the cache
            // file stays, for the picker that Rust then asks for.
            val src = File(f[3])
            val ticket = f[1].toLong()
            val uri = Uri.parse(f[2])
            try {
                contentResolver.openOutputStream(uri, "wt")!!.use { out -> src.inputStream().use { it.copyTo(out) } }
                src.delete()
                nativeSaveDone(ticket, 1, f[2], displayName(uri))
            } catch (e: Exception) {
                nativeSaveDone(ticket, -1, e.message ?: e.toString(), "")
            }
            nextFile()
            return
        }
        if (f[0] == "read") {
            // Recent asked for a picked document: copy its bytes out, then open them like a picked one.
            val name = f[2]
            try {
                val dest = File(cacheDir, "open-${System.nanoTime()}.bin")
                contentResolver.openInputStream(Uri.parse(f[1]))?.use { input -> dest.outputStream().use { input.copyTo(it) } }
                    ?: throw java.io.FileNotFoundException(f[1])
                nativeOpened(name, dest.path, f[1])
            } catch (e: Exception) {
                nativeNotice(readFailure(name, e))
            }
            nextFile()
            return
        }
        if (f[0] == "print") {
            printPdf(File(f[1]))
            nextFile()
            return
        }
        filesBusy = true
        if (f[0] == "save") {
            saveTicket = f[1].toLong()
            saveSource = f[3]
            val name = f[2]
            val mime = android.webkit.MimeTypeMap.getSingleton().getMimeTypeFromExtension(name.substringAfterLast('.', "").lowercase())
            createDoc.launch(Pair(name, mime ?: "application/octet-stream"))
        } else {
            val types = if (f.getOrNull(1) == "picture") arrayOf("image/*") else arrayOf("*/*")
            openDoc.launch(types)
        }
    }

    /** Why a Recent document would not open, in words the status line can show. */
    private fun readFailure(name: String, e: Exception): String = when (e) {
        is java.io.FileNotFoundException -> "$name is no longer there. It was moved, renamed or deleted."
        is SecurityException -> "WordCraft can no longer open $name. Open it again from File > Open."
        else -> "Couldn't open $name: ${e.message ?: e.toString()}"
    }

    /** The system print dialog for a PDF file; the file is deleted when the dialog closes. */
    private fun printPdf(pdf: File) {
        val adapter = object : PrintDocumentAdapter() {
            override fun onLayout(old: PrintAttributes?, new: PrintAttributes, cancel: CancellationSignal?, cb: LayoutResultCallback, extras: Bundle?) {
                if (cancel?.isCanceled == true) return cb.onLayoutCancelled()
                cb.onLayoutFinished(PrintDocumentInfo.Builder("WordCraft.pdf").setContentType(PrintDocumentInfo.CONTENT_TYPE_DOCUMENT).build(), old != new)
            }

            override fun onWrite(pages: Array<out PageRange>, dest: ParcelFileDescriptor, cancel: CancellationSignal?, cb: WriteResultCallback) {
                try {
                    ParcelFileDescriptor.AutoCloseOutputStream(dest).use { out -> pdf.inputStream().use { it.copyTo(out) } }
                    cb.onWriteFinished(arrayOf(PageRange.ALL_PAGES))
                } catch (e: Exception) {
                    cb.onWriteFailed(e.message)
                }
            }

            override fun onFinish() {
                pdf.delete()
            }
        }
        (getSystemService(Context.PRINT_SERVICE) as PrintManager).print("WordCraft", adapter, null)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val content = findViewById<View>(android.R.id.content)
        ViewCompat.setOnApplyWindowInsetsListener(content) { v, insets ->
            val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout() or WindowInsetsCompat.Type.ime())
            v.setPadding(bars.left, bars.top, bars.right, bars.bottom)
            // The native side gets touch positions in window pixels, not relative to the padded surface.
            nativeSetInsets(bars.left, bars.top)
            WindowInsetsCompat.CONSUMED
        }
        imeView = ImeView()
        (content as FrameLayout).addView(imeView, FrameLayout.LayoutParams(1, 1))
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                // Back closes the innermost open dialog, sheet page, sheet, menu or Backstage first.
                if (nativeCanGoBack()) nativeIme(4, "", KeyEvent.KEYCODE_ESCAPE, 0) else leave()
            }
        })
        clipboard.addPrimaryClipChangedListener(clipListener)
        reportNight(resources.configuration)
        handler.post(keyboardPoll)
    }

    // GameActivity passes key events to the native side before the back dispatcher sees them.
    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        if (event.keyCode == KeyEvent.KEYCODE_BACK) {
            if (event.action == KeyEvent.ACTION_UP) {
                if (nativeCanGoBack()) nativeIme(4, "", KeyEvent.KEYCODE_ESCAPE, 0) else leave()
            }
            return true
        }
        return super.dispatchKeyEvent(event)
    }

    /**
     * Back with nothing open: go to the background instead of finishing. The process (and the
     * open document) stays; eframe can run only once per process, so a finished activity would
     * leave the next launch without a running UI loop.
     */
    private fun leave() {
        moveTaskToBack(true)
    }

    private val clipboard by lazy { getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager }
    private val clipListener = ClipboardManager.OnPrimaryClipChangedListener { readClip() }

    /** Hand the system clip's text to Rust, which Paste reads. Android only lets the focused app read it. */
    private fun readClip() {
        val text = clipboard.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)?.coerceToText(this)?.toString() ?: return
        nativeClip(text)
    }

    // configChanges includes uiMode, so a dark mode switch arrives here instead of recreating the activity.
    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        reportNight(newConfig)
    }

    private fun reportNight(c: Configuration) {
        val night = c.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
        // The system-bar padding shows the window background, which the Light theme keeps light:
        // match it to the header colour and flip the bar icons so they stay readable.
        window.decorView.setBackgroundColor(if (night) Color.rgb(18, 19, 23) else Color.rgb(250, 250, 250))
        WindowCompat.getInsetsController(window, window.decorView).run {
            isAppearanceLightStatusBars = !night
            isAppearanceLightNavigationBars = !night
        }
        nativeSetNight(night)
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) readClip()
    }

    // Android may kill a backgrounded process without warning: copy unsaved work now (Rust, recovery).
    override fun onPause() {
        super.onPause()
        nativeFlushRecovery()
    }

    override fun onStop() {
        super.onStop()
        nativeFlushRecovery()
    }

    override fun onDestroy() {
        clipboard.removePrimaryClipChangedListener(clipListener)
        handler.removeCallbacks(keyboardPoll)
        super.onDestroy()
    }

    private fun showKeyboard() {
        imeView.requestFocus()
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        imm.restartInput(imeView)
        imm.showSoftInput(imeView, 0)
    }

    private fun hideKeyboard() {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        imm.hideSoftInputFromWindow(imeView.windowToken, 0)
    }

    /** Rust side (android.rs): kind 0 commit, 1 composing text, 2 finish, 3 delete surrounding, 4 key. */
    private external fun nativeIme(kind: Int, text: String, a: Int, b: Int)

    /** Left and top padding in pixels that touch positions must be shifted by. */
    private external fun nativeSetInsets(left: Int, top: Int)

    /** Next file request from Rust, or an empty string. */
    private external fun nativeTakeRequest(): String

    /** A save ended: status 1 saved ([msg] is the document URI, [display] its name), 0 cancelled, else failed with [msg]. */
    private external fun nativeSaveDone(ticket: Long, status: Int, msg: String, display: String)

    /** A file operation failed: say [msg] on the status line. */
    private external fun nativeNotice(msg: String)

    /** The user picked the document at [uri]; its bytes are in the cache file at [path]. */
    private external fun nativeOpened(name: String, path: String, uri: String)

    /** Next copy for the system clipboard, prefixed with "c", or an empty string. */
    private external fun nativeTakeClip(): String

    /** Whether the system is in night mode. */
    private external fun nativeSetNight(night: Boolean)

    /** The system clipboard's text. */
    private external fun nativeClip(text: String)

    /** The activity is being backgrounded: write a recovery copy of unsaved work. */
    private external fun nativeFlushRecovery()

    /** Whether egui wants the soft keyboard. */
    private external fun nativeWantsKeyboard(): Boolean

    /** Whether Android Back has a sheet, menu or Backstage to close. */
    private external fun nativeCanGoBack(): Boolean

    private inner class ImeView : View(this@MainActivity) {
        init {
            isFocusable = true
            isFocusableInTouchMode = true
        }

        override fun onCheckIsTextEditor(): Boolean = true

        override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
            outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE or InputType.TYPE_TEXT_FLAG_AUTO_CORRECT
            outAttrs.imeOptions = EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_FLAG_NO_EXTRACT_UI or EditorInfo.IME_ACTION_NONE
            return Connection(this)
        }
    }

    private inner class Connection(view: View) : BaseInputConnection(view, true) {
        override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
            nativeIme(0, text?.toString() ?: "", 0, 0)
            return true
        }

        override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
            nativeIme(1, text?.toString() ?: "", 0, 0)
            return true
        }

        override fun finishComposingText(): Boolean {
            nativeIme(2, "", 0, 0)
            return true
        }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
            nativeIme(3, "", beforeLength, afterLength)
            return true
        }

        override fun deleteSurroundingTextInCodePoints(beforeLength: Int, afterLength: Int): Boolean =
            deleteSurroundingText(beforeLength, afterLength)

        override fun sendKeyEvent(event: KeyEvent): Boolean {
            if (event.action == KeyEvent.ACTION_DOWN) nativeIme(4, "", event.keyCode, 0)
            return true
        }

        override fun performEditorAction(actionCode: Int): Boolean {
            nativeIme(4, "", KeyEvent.KEYCODE_ENTER, 0)
            return true
        }

        // ponytail: no surrounding text is reported, so Gboard's suggestions and autocorrect use
        // only the composing word. Add the paragraph text from Rust when this is not enough.
        override fun getTextBeforeCursor(n: Int, flags: Int): CharSequence = ""
        override fun getTextAfterCursor(n: Int, flags: Int): CharSequence = ""
        override fun getSelectedText(flags: Int): CharSequence? = null
    }
}
