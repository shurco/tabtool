package app.shurco.tabtool

import android.app.Activity
import android.content.Intent
import android.net.Uri
import java.io.File
import java.io.IOException

/**
 * SAF import (`ACTION_OPEN_DOCUMENT`) and export (`ACTION_CREATE_DOCUMENT`).
 *
 * State is guarded by [TabToolBridge.lock]; copies run on
 * [TabToolBridge.copyExecutor].
 */
internal object DocumentPicker {
    private const val REQUEST_IMPORT = 1001
    private const val REQUEST_EXPORT = 1002

    @Volatile
    var importPending = false
        private set

    @Volatile
    private var importAwaitingDrain = false

    @Volatile
    private var importResult: String? = null

    @Volatile
    var exportPending = false
        private set

    @Volatile
    private var exportAwaitingDrain = false

    @Volatile
    private var exportResult: String? = null

    @Volatile
    private var exportSource: File? = null

    /** Clears every pending operation. The caller holds [TabToolBridge.lock]. */
    fun reset() {
        importPending = false
        importAwaitingDrain = false
        importResult = null
        exportPending = false
        exportAwaitingDrain = false
        exportResult = null
        exportSource = null
    }

    fun importDocument(): String? {
        synchronized(TabToolBridge.lock) {
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
        val activity = TabToolBridge.activity ?: return failure("The activity is gone")
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

    fun exportDocument(sourcePath: String, suggestedName: String): String? {
        synchronized(TabToolBridge.lock) {
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
        val activity = TabToolBridge.activity ?: return failure("The activity is gone")
        val source = File(sourcePath)
        if (!source.isFile || !source.canRead()) {
            return failure("The source document cannot be read")
        }
        val name =
            DocumentFiles.safeName(suggestedName)
                ?: return failure("The suggested file name is invalid")
        exportSource = source
        exportPending = true
        activity.runOnUiThread {
            val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = DocumentFiles.mimeTypeFor(name)
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
        val activity = TabToolBridge.activity
        if (activity == null) {
            finishImport(failure("The activity is gone"))
            return
        }
        TabToolBridge.copyExecutor.execute {
            try {
                val name = DocumentFiles.copyIntoDocuments(activity, uri)
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
        val activity = TabToolBridge.activity
        val source = exportSource
        if (activity == null || source == null) {
            finishExport(failure("The source document is missing"))
            return
        }
        TabToolBridge.copyExecutor.execute {
            try {
                activity.contentResolver.openOutputStream(uri, "wt")?.use { output ->
                    source.inputStream().use { input ->
                        input.copyTo(output, DocumentFiles.COPY_BUFFER)
                    }
                } ?: throw IOException("The destination document cannot be written")
                finishExport(OK)
            } catch (error: IOException) {
                finishExport(failure(error.message ?: "The copy could not be written"))
            } catch (error: SecurityException) {
                finishExport(failure(error.message ?: "Access to the destination was denied"))
            }
        }
    }

    private fun finishImport(result: String) {
        synchronized(TabToolBridge.lock) {
            importResult = result
            importPending = false
            importAwaitingDrain = true
        }
        TabToolBridge.nativeWake()
    }

    private fun finishExport(result: String) {
        synchronized(TabToolBridge.lock) {
            exportResult = result
            exportPending = false
            exportAwaitingDrain = true
            exportSource = null
        }
        TabToolBridge.nativeWake()
    }
}
