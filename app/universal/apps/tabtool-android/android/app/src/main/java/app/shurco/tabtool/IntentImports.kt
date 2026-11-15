package app.shurco.tabtool

import android.app.Activity
import android.content.Intent
import android.net.Uri
import java.io.File
import java.io.FileOutputStream
import java.io.IOException

/**
 * Documents other apps hand to TabTool through `ACTION_VIEW` / `ACTION_SEND`.
 *
 * The result queue is guarded by [TabToolBridge.lock]; copies run on
 * [TabToolBridge.copyExecutor].
 */
internal object IntentImports {
    private const val INTENT_STAGING_PREFIX = ".incoming-"

    private val intentResults = ArrayDeque<String>()

    private var intentSequence = 0

    /** Drops queued results. The caller holds [TabToolBridge.lock]. */
    fun reset() {
        intentResults.clear()
    }

    fun handleIntent(intent: Intent?) {
        if (intent == null) return
        val action = intent.action
        if (action != Intent.ACTION_VIEW && action != Intent.ACTION_SEND) return
        val uri =
            if (action == Intent.ACTION_SEND) {
                @Suppress("DEPRECATION")
                intent.getParcelableExtra(Intent.EXTRA_STREAM) as? Uri
            } else {
                intent.data
            }
        if (uri == null) {
            finishIntent(failure("No file was shared"))
            return
        }
        val activity = TabToolBridge.activity ?: return
        TabToolBridge.copyExecutor.execute {
            try {
                val staged = stageIntent(activity, uri, intent.type)
                if (staged == null) {
                    finishIntent(failure("The file type is not supported"))
                } else {
                    val (file, displayName) = staged
                    finishIntent(ok("${file.name}\n$displayName"))
                }
            } catch (error: IOException) {
                finishIntent(failure(error.message ?: "The document could not be copied"))
            } catch (error: SecurityException) {
                finishIntent(failure(error.message ?: "Access to the document was denied"))
            }
        }
    }

    fun takeIntentImport(): String? {
        synchronized(TabToolBridge.lock) {
            return intentResults.removeFirstOrNull()
        }
    }

    /**
     * Copies an intent stream into a hidden staging file.
     *
     * Returns the staged file and the display name, or `null` when neither the
     * name nor the MIME type names a supported table file.
     */
    private fun stageIntent(activity: Activity, uri: Uri, mime: String?): Pair<File, String>? {
        val original =
            DocumentFiles.displayName(activity, uri) ?: uri.lastPathSegment?.substringAfterLast('/')
        val extension = intentExtension(mime, original) ?: return null
        val base =
            DocumentFiles.safeName(original ?: "Imported.$extension") ?: "Imported.$extension"
        val dot = base.lastIndexOf('.')
        val stem = if (dot > 0) base.substring(0, dot) else base
        val displayName =
            if (base.substringAfterLast('.', "").lowercase() in DocumentFiles.tableExtensions) {
                base
            } else {
                "$stem.$extension"
            }
        intentSequence += 1
        val destination =
            DocumentFiles.uniqueDestination(
                DocumentFiles.documentsDir(activity),
                "$INTENT_STAGING_PREFIX${intentSequence}.$extension",
            )
        val partial = File(destination.parentFile, "${destination.name}.part")
        try {
            activity.contentResolver.openInputStream(uri)?.use { input ->
                FileOutputStream(partial).use { output ->
                    input.copyTo(output, DocumentFiles.COPY_BUFFER)
                }
            } ?: throw IOException("The shared document cannot be opened")
            if (!partial.renameTo(destination)) {
                throw IOException("The shared document could not be copied")
            }
            return destination to displayName
        } finally {
            partial.delete()
        }
    }

    private fun intentExtension(mime: String?, displayName: String?): String? {
        val named =
            displayName?.let { DocumentFiles.safeName(it) }
                ?.substringAfterLast('.', "")
                ?.lowercase()
        if (named != null && named in DocumentFiles.tableExtensions) {
            return named
        }
        return when (mime?.lowercase()) {
            "text/csv", "text/comma-separated-values", "application/csv" -> "csv"
            "text/tab-separated-values" -> "tsv"
            "text/plain" -> "txt"
            else -> null
        }
    }

    private fun finishIntent(result: String) {
        synchronized(TabToolBridge.lock) {
            intentResults.addLast(result)
        }
        TabToolBridge.nativeWake()
    }
}
