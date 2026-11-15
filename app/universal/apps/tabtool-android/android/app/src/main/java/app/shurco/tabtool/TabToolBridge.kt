package app.shurco.tabtool

import android.app.Activity
import android.content.Intent
import android.content.res.Configuration
import androidx.core.graphics.Insets
import androidx.core.view.WindowInsetsCompat
import java.util.TimeZone
import java.util.concurrent.Executors
import kotlin.math.roundToInt

/**
 * JNI surface for the Rust platform layer. Every `@JvmStatic` method is called
 * from the native thread that runs `android_main`.
 *
 * Results are one of `OK`, `OK:<value>`, `CANCELLED` or `ERROR:<message>`.
 * SAF operations answer `null` while the picker is open; the next call returns
 * the encoded result. Copies stream in chunks, so a large document is never
 * read into memory.
 */
object TabToolBridge {
    internal val lock = Any()
    internal val copyExecutor = Executors.newSingleThreadExecutor()

    @Volatile
    internal var activity: Activity? = null
        private set

    @Volatile
    private var insetsDp: IntArray = intArrayOf(0, 0, 0, 0, 0)

    @Volatile
    private var pendingBack = false

    @Volatile
    private var backgroundPending = false

    fun attach(activity: Activity) {
        this.activity = activity
    }

    fun detach() {
        TextEditorHost.detach()
        synchronized(lock) {
            DocumentPicker.reset()
            pendingBack = false
            backgroundPending = false
            IntentImports.reset()
        }
        activity = null
    }

    /** Records a transition; the native app polls [takeBackgroundEvent]. */
    fun updateForeground(foreground: Boolean) {
        if (!foreground) {
            backgroundPending = true
            nativeSaveOnBackground()
        }
    }

    @JvmStatic
    fun takeBackgroundEvent(): Boolean {
        val pending = backgroundPending
        backgroundPending = false
        return pending
    }

    /**
     * Queues one document another app wants TabTool to open.
     *
     * The stream is copied into a hidden staging file in the private documents
     * directory; the native app picks the final name and opens it. Only
     * `ACTION_VIEW` and `ACTION_SEND` with a stream are accepted.
     */
    fun handleIntent(intent: Intent?) = IntentImports.handleIntent(intent)

    /**
     * Returns `OK:<staging file name>\n<display name>` once a copy finishes.
     *
     * Several VIEW/SEND intents can arrive before the native loop polls; each
     * copy is queued, so no document is dropped or left as an orphan.
     */
    @JvmStatic
    fun takeIntentImport(): String? = IntentImports.takeIntentImport()

    /** The app's private data directory, for settings and process-death state. */
    @JvmStatic
    fun filesDir(): String? = activity?.filesDir?.absolutePath

    /** The device's UTC offset in minutes and 0 for a 24-hour clock. */
    @JvmStatic
    fun clock(): IntArray {
        val activity = activity ?: return intArrayOf(0, 0)
        val offset = TimeZone.getDefault().getOffset(System.currentTimeMillis()) / 60_000
        val twelveHour = if (android.text.format.DateFormat.is24HourFormat(activity)) 0 else 1
        return intArrayOf(offset, twelveHour)
    }

    @JvmStatic
    fun isImportPending(): Boolean = DocumentPicker.importPending

    @JvmStatic
    fun isExportPending(): Boolean = DocumentPicker.exportPending

    fun updateInsets(insets: WindowInsetsCompat) {
        val density = activity?.resources?.displayMetrics?.density?.takeIf { it > 0f } ?: return
        val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
        val cutout = insets.getInsets(WindowInsetsCompat.Type.displayCutout())
        val safe = Insets.max(bars, cutout)
        val ime = insets.getInsets(WindowInsetsCompat.Type.ime())

        insetsDp = intArrayOf(
            dp(safe.left, density),
            dp(safe.top, density),
            dp(safe.right, density),
            dp(safe.bottom, density),
            dp(ime.bottom, density),
        )
    }

    private fun dp(px: Int, density: Float): Int = (px / density).roundToInt()

    @JvmStatic
    fun clipboardGet(): String? {
        val activity = activity ?: return null
        return SystemActions.clipboardGet(activity)
    }

    @JvmStatic
    fun clipboardSet(text: String) {
        val activity = activity ?: return
        SystemActions.clipboardSet(activity, text)
    }

    @JvmStatic
    fun insets(): IntArray = insetsDp.copyOf()

    /** Whether the system is in night mode; winit does not report it. */
    @JvmStatic
    fun isDarkMode(): Boolean {
        val activity = activity ?: return false
        val mode = activity.resources.configuration.uiMode and
            Configuration.UI_MODE_NIGHT_MASK
        return mode == Configuration.UI_MODE_NIGHT_YES
    }

    /** The app-private directory that holds working documents. */
    @JvmStatic
    fun documentsDir(): String? {
        val activity = activity ?: return null
        return DocumentFiles.documentsDir(activity).absolutePath
    }

    /**
     * Starts `ACTION_OPEN_DOCUMENT` and copies the picked document into the
     * private directory. Returns `null` while the picker or copy is running;
     * the next call returns `OK:<file name>`, `CANCELLED` or `ERROR:<message>`.
     */
    @JvmStatic
    fun importDocument(): String? = DocumentPicker.importDocument()

    /**
     * Starts `ACTION_CREATE_DOCUMENT` and streams a copy of `sourcePath`.
     * Returns `null` while the picker or copy is running; the next call returns
     * `OK`, `CANCELLED` or `ERROR:<message>`.
     */
    @JvmStatic
    fun exportDocument(sourcePath: String, suggestedName: String): String? =
        DocumentPicker.exportDocument(sourcePath, suggestedName)

    /** Opens the system share sheet for a private document. */
    @JvmStatic
    fun shareFile(path: String): String {
        val activity = activity ?: return failure("The activity is gone")
        return SystemActions.shareFile(activity, path)
    }

    /** The system's preferred language tags, most preferred first. */
    @JvmStatic
    fun languages(): Array<String> {
        val activity = activity ?: return emptyArray()
        val locales = activity.resources.configuration.locales
        return (0 until locales.size()).map { locales.get(it).toLanguageTag() }.toTypedArray()
    }

    /** Opens an http, https or mailto link in another app. */
    @JvmStatic
    fun openUrl(url: String): String {
        val activity = activity ?: return failure("The activity is gone")
        return SystemActions.openUrl(activity, url)
    }

    /**
     * Handles a system back press: a visible editor is cancelled, otherwise the
     * event is queued for the native app. Always consumed so the activity only
     * finishes when the app calls [finishActivity].
     */
    fun handleBack(): Boolean {
        if (cancelTextEditor()) {
            return true
        }
        pendingBack = true
        return true
    }

    @JvmStatic
    fun takeBack(): Boolean {
        val back = pendingBack
        pendingBack = false
        return back
    }

    @JvmStatic
    fun finishActivity() {
        val activity = activity ?: return
        activity.runOnUiThread { activity.finish() }
    }

    @JvmStatic
    fun showTextEditor(
        left: Float,
        top: Float,
        width: Float,
        height: Float,
        text: String,
        multiline: Boolean,
    ): Boolean = TextEditorHost.show(left, top, width, height, text, multiline)

    @JvmStatic
    fun hideTextEditor() = TextEditorHost.hide()

    /** Commits the visible editor's current text through the JNI callback. */
    @JvmStatic
    fun commitTextEditor() = TextEditorHost.commit()

    /** Moves the visible editor without resetting its text. */
    @JvmStatic
    fun moveTextEditor(
        left: Float,
        top: Float,
        width: Float,
        height: Float,
    ) = TextEditorHost.move(left, top, width, height)

    /** Returns true when a visible editor was cancelled. Runs on the UI thread. */
    fun cancelTextEditor(): Boolean = TextEditorHost.cancel()

    /** Called by the native editor when the user commits. */
    external fun nativeTextCommitted(text: String)

    /** Called by the native editor when the user cancels or it cannot open. */
    external fun nativeTextCancelled()

    /** Wakes the egui pass after a callback that must be observed promptly. */
    external fun nativeWake()

    /** Flushes the pending edit before the activity pauses. */
    external fun nativeSaveOnBackground(): Boolean

    fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) =
        DocumentPicker.onActivityResult(requestCode, resultCode, data)
}
