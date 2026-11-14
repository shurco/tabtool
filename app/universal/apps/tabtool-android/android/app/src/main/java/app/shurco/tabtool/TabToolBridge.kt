package app.shurco.tabtool

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import android.widget.FrameLayout
import androidx.core.content.FileProvider
import androidx.core.graphics.Insets
import androidx.core.view.WindowInsetsCompat
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
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
    private const val REQUEST_IMPORT = 1001
    private const val REQUEST_EXPORT = 1002
    private const val OK = "OK"
    private const val CANCELLED = "CANCELLED"
    private const val ERROR = "ERROR:"
    private const val COPY_BUFFER = 64 * 1024
    private const val MAX_FILE_NAME = 120
    private val tableExtensions = setOf("csv", "tsv", "txt", "tab", "dat")

    private val lock = Any()
    private val copyExecutor = Executors.newSingleThreadExecutor()

    @Volatile
    private var activity: Activity? = null

    @Volatile
    private var textEditor: TabToolTextEditor? = null

    @Volatile
    private var insetsDp: IntArray = intArrayOf(0, 0, 0, 0, 0)

    @Volatile
    private var importPending = false

    @Volatile
    private var importAwaitingDrain = false

    @Volatile
    private var importResult: String? = null

    @Volatile
    private var exportPending = false

    @Volatile
    private var exportAwaitingDrain = false

    @Volatile
    private var exportResult: String? = null

    @Volatile
    private var exportSource: File? = null

    @Volatile
    private var pendingBack = false

    fun attach(activity: Activity) {
        this.activity = activity
    }

    fun detach() {
        textEditor?.dismissWithoutCallback()
        textEditor = null
        synchronized(lock) {
            importPending = false
            importAwaitingDrain = false
            importResult = null
            exportPending = false
            exportAwaitingDrain = false
            exportResult = null
            exportSource = null
            pendingBack = false
        }
        activity = null
    }

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
        return try {
            val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val clip = clipboard.primaryClip ?: return null
            if (clip.itemCount == 0) null else clip.getItemAt(0).text?.toString()
        } catch (error: SecurityException) {
            null
        }
    }

    @JvmStatic
    fun clipboardSet(text: String) {
        val activity = activity ?: return
        try {
            val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("TabTool", text))
        } catch (error: SecurityException) {
            // Copying is best effort; the app keeps its own value.
        }
    }

    @JvmStatic
    fun insets(): IntArray = insetsDp.copyOf()

    /** The app-private directory that holds working documents. */
    @JvmStatic
    fun documentsDir(): String? {
        val activity = activity ?: return null
        return documentsDir(activity).absolutePath
    }

    private fun documentsDir(activity: Activity): File =
        File(activity.filesDir, "documents").apply { mkdirs() }

    /**
     * Starts `ACTION_OPEN_DOCUMENT` and copies the picked document into the
     * private directory. Returns `null` while the picker or copy is running;
     * the next call returns `OK:<file name>`, `CANCELLED` or `ERROR:<message>`.
     */
    @JvmStatic
    fun importDocument(): String? {
        synchronized(lock) {
            if (importAwaitingDrain) {
                importAwaitingDrain = false
                val result = importResult
                importResult = null
                if (result != null) {
                    return result
                }
            }
        }
        if (importPending) {
            return null
        }
        val activity = activity ?: return failure("The activity is gone")
        importPending = true
        activity.runOnUiThread {
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "text/*"
            }
            try {
                activity.startActivityForResult(intent, REQUEST_IMPORT)
            } catch (error: Exception) {
                finishImport(failure(error.message ?: "No file picker is available"))
            }
        }
        return null
    }

    /**
     * Starts `ACTION_CREATE_DOCUMENT` and streams a copy of `sourcePath`.
     * Returns `null` while the picker or copy is running; the next call returns
     * `OK`, `CANCELLED` or `ERROR:<message>`.
     */
    @JvmStatic
    fun exportDocument(sourcePath: String, suggestedName: String): String? {
        synchronized(lock) {
            if (exportAwaitingDrain) {
                exportAwaitingDrain = false
                val result = exportResult
                exportResult = null
                if (result != null) {
                    return result
                }
            }
        }
        if (exportPending) {
            return null
        }
        val activity = activity ?: return failure("The activity is gone")
        val source = File(sourcePath)
        if (!source.isFile || !source.canRead()) {
            return failure("The source document cannot be read")
        }
        val name = safeName(suggestedName) ?: return failure("The suggested file name is invalid")
        exportSource = source
        exportPending = true
        activity.runOnUiThread {
            val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = mimeTypeFor(name)
                putExtra(Intent.EXTRA_TITLE, name)
            }
            try {
                activity.startActivityForResult(intent, REQUEST_EXPORT)
            } catch (error: Exception) {
                finishExport(failure(error.message ?: "No file picker is available"))
            }
        }
        return null
    }

    /** Opens the system share sheet for a private document. */
    @JvmStatic
    fun shareFile(path: String): String {
        val activity = activity ?: return failure("The activity is gone")
        val file = File(path)
        if (!file.isFile) {
            return failure("The document is missing")
        }
        return try {
            val uri =
                FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", file)
            val send = Intent(Intent.ACTION_SEND).apply {
                type = activity.contentResolver.getType(uri) ?: "text/plain"
                putExtra(Intent.EXTRA_STREAM, uri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            activity.startActivity(Intent.createChooser(send, null))
            OK
        } catch (error: IllegalArgumentException) {
            failure(error.message ?: "The document cannot be shared")
        } catch (error: ActivityNotFoundException) {
            failure("No app can share this document")
        }
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
        val uri = Uri.parse(url)
        val scheme = uri.scheme?.lowercase()
        if (scheme != "http" && scheme != "https" && scheme != "mailto") {
            return failure("Unsupported link")
        }
        return try {
            activity.startActivity(Intent(Intent.ACTION_VIEW, uri))
            OK
        } catch (error: ActivityNotFoundException) {
            failure("No app can open this link")
        } catch (error: SecurityException) {
            failure(error.message ?: "The link cannot be opened")
        }
    }

    @JvmStatic
    fun isDocumentOperationPending(): Boolean = importPending || exportPending

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
    ): Boolean {
        val activity = activity ?: return false
        if (!width.isFinite() || !height.isFinite() || width <= 0f || height <= 0f) {
            return false
        }
        activity.runOnUiThread {
            val root = activity.findViewById<FrameLayout>(android.R.id.content)
            if (root == null) {
                nativeTextCancelled()
                return@runOnUiThread
            }
            val density = activity.resources.displayMetrics.density
            val editor = textEditor ?: TabToolTextEditor(activity).also { textEditor = it }
            val params =
                FrameLayout.LayoutParams(
                    (width * density).roundToInt(),
                    (height * density).roundToInt(),
                ).apply {
                    leftMargin = (left * density).roundToInt()
                    topMargin = (top * density).roundToInt()
                }
            if (editor.parent == null) {
                root.addView(editor, params)
            } else {
                editor.layoutParams = params
            }
            editor.elevation = 24f
            editor.configure(text, multiline)
        }
        return true
    }

    @JvmStatic
    fun hideTextEditor() {
        val activity = activity ?: return
        activity.runOnUiThread {
            textEditor?.dismissWithoutCallback()
            textEditor = null
        }
    }

    /** Commits the visible editor's current text through the JNI callback. */
    @JvmStatic
    fun commitTextEditor() {
        val activity = activity ?: return
        activity.runOnUiThread { textEditor?.commitEdit() }
    }

    /** Moves the visible editor without resetting its text. */
    @JvmStatic
    fun moveTextEditor(
        left: Float,
        top: Float,
        width: Float,
        height: Float,
    ) {
        val activity = activity ?: return
        if (!width.isFinite() || !height.isFinite() || width <= 0f || height <= 0f) {
            return
        }
        activity.runOnUiThread {
            val editor = textEditor ?: return@runOnUiThread
            val density = activity.resources.displayMetrics.density
            val params =
                FrameLayout.LayoutParams(
                    (width * density).roundToInt(),
                    (height * density).roundToInt(),
                ).apply {
                    leftMargin = (left * density).roundToInt()
                    topMargin = (top * density).roundToInt()
                }
            editor.layoutParams = params
            editor.requestLayout()
        }
    }

    /** Returns true when a visible editor was cancelled. Runs on the UI thread. */
    fun cancelTextEditor(): Boolean {
        val editor = textEditor ?: return false
        if (editor.parent == null) {
            textEditor = null
            return false
        }
        editor.cancelEdit()
        textEditor = null
        return true
    }

    /** Called by the native editor when the user commits. */
    external fun nativeTextCommitted(text: String)

    /** Called by the native editor when the user cancels or it cannot open. */
    external fun nativeTextCancelled()

    /** Spike-only self-test: proves the Rust bridge clears JNI exceptions. */
    @JvmStatic
    fun throwSecurityException() {
        throw SecurityException("TabTool JNI exception test")
    }

    fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        when (requestCode) {
            REQUEST_IMPORT -> onImportResult(resultCode, data?.data)
            REQUEST_EXPORT -> onExportResult(resultCode, data?.data)
        }
    }

    private fun onImportResult(resultCode: Int, uri: Uri?) {
        if (resultCode != Activity.RESULT_OK || uri == null) {
            finishImport(CANCELLED)
            return
        }
        val activity = activity
        if (activity == null) {
            finishImport(failure("The activity is gone"))
            return
        }
        copyExecutor.execute {
            try {
                val name = copyIntoDocuments(activity, uri)
                finishImport(if (name == null) failure("The document could not be copied") else ok(name))
            } catch (error: IOException) {
                finishImport(failure(error.message ?: "The document could not be copied"))
            } catch (error: SecurityException) {
                finishImport(failure(error.message ?: "Access to the document was denied"))
            }
        }
    }

    private fun onExportResult(resultCode: Int, uri: Uri?) {
        if (resultCode != Activity.RESULT_OK || uri == null) {
            finishExport(CANCELLED)
            return
        }
        val activity = activity
        val source = exportSource
        if (activity == null || source == null) {
            finishExport(failure("The source document is missing"))
            return
        }
        copyExecutor.execute {
            try {
                activity.contentResolver.openOutputStream(uri, "wt")?.use { output ->
                    source.inputStream().use { input -> input.copyTo(output, COPY_BUFFER) }
                } ?: throw IOException("The destination document cannot be written")
                finishExport(OK)
            } catch (error: IOException) {
                finishExport(failure(error.message ?: "The copy could not be written"))
            } catch (error: SecurityException) {
                finishExport(failure(error.message ?: "Access to the destination was denied"))
            }
        }
    }

    /** Streams a content URI into the private directory and returns the new name. */
    private fun copyIntoDocuments(activity: Activity, uri: Uri): String? {
        val displayName = displayName(activity, uri) ?: uri.lastPathSegment?.substringAfterLast('/')
        val name = safeName(displayName ?: "Imported") ?: throw IOException("The file name is invalid")
        if (name.substringAfterLast('.', "").lowercase() !in tableExtensions) {
            throw IOException("The file type is not supported")
        }
        val destination = uniqueDestination(documentsDir(activity), name)
        val partial = File(destination.parentFile, "${destination.name}.part")
        try {
            activity.contentResolver.openInputStream(uri)?.use { input ->
                FileOutputStream(partial).use { output -> input.copyTo(output, COPY_BUFFER) }
            } ?: throw IOException("The picked document cannot be opened")
            if (!partial.renameTo(destination)) {
                throw IOException("The import could not be finished")
            }
            return destination.name
        } finally {
            partial.delete()
        }
    }

    private fun displayName(activity: Activity, uri: Uri): String? {
        val cursor =
            activity.contentResolver.query(
                uri,
                arrayOf(OpenableColumns.DISPLAY_NAME),
                null,
                null,
                null,
            ) ?: return null
        cursor.use {
            if (!it.moveToFirst()) {
                return null
            }
            val column = it.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            return if (column >= 0) it.getString(column) else null
        }
    }

    /** A bounded file name with path separators and control characters removed. */
    private fun safeName(name: String): String? {
        val base = File(name).name
            .replace('/', '_')
            .replace('\\', '_')
            .map { if (it.isISOControl()) '_' else it }
            .joinToString("")
            .trim()
        if (base.isEmpty() || base == "." || base == "..") {
            return null
        }
        if (base.length <= MAX_FILE_NAME) {
            return base
        }
        val dot = base.lastIndexOf('.')
        val extension = if (dot > 0) base.substring(dot) else ""
        val suffix = if (extension.length <= 10) extension else ""
        val stem = if (dot > 0) base.substring(0, dot) else base
        return stem.take(MAX_FILE_NAME - suffix.length).trimEnd() + suffix
    }

    /** `directory/name`, or `directory/name 2.ext` when taken (case-insensitive). */
    private fun uniqueDestination(directory: File, name: String): File {
        val taken = directory.list()?.map { it.lowercase() }?.toSet().orEmpty()
        if (!taken.contains(name.lowercase())) {
            return File(directory, name)
        }
        val dot = name.lastIndexOf('.')
        val stem = if (dot > 0) name.substring(0, dot) else name
        val extension = if (dot > 0) name.substring(dot) else ""
        var index = 2
        while (true) {
            val candidate = "$stem $index$extension"
            if (!taken.contains(candidate.lowercase())) {
                return File(directory, candidate)
            }
            index++
        }
    }

    private fun mimeTypeFor(name: String): String =
        when (name.substringAfterLast('.', "").lowercase()) {
            "csv" -> "text/csv"
            "tsv", "tab" -> "text/tab-separated-values"
            "dat" -> "application/octet-stream"
            else -> "text/plain"
        }

    private fun ok(value: String? = null): String =
        if (value.isNullOrEmpty()) OK else "$OK:$value"

    private fun failure(message: String): String =
        "$ERROR${message.replace('\n', ' ').replace('\r', ' ').take(300)}"

    private fun finishImport(result: String) {
        synchronized(lock) {
            importResult = result
            importPending = false
            importAwaitingDrain = true
        }
    }

    private fun finishExport(result: String) {
        synchronized(lock) {
            exportResult = result
            exportPending = false
            exportAwaitingDrain = true
            exportSource = null
        }
    }
}
