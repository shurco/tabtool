package app.shurco.tabtool

import android.app.Activity
import android.net.Uri
import android.provider.OpenableColumns
import java.io.File
import java.io.FileOutputStream
import java.io.IOException

/** The private documents directory: naming, uniqueness and streamed copies into it. */
internal object DocumentFiles {
    const val COPY_BUFFER = 64 * 1024
    private const val MAX_FILE_NAME = 120
    val tableExtensions = setOf("csv", "tsv", "txt", "tab", "dat")

    fun documentsDir(activity: Activity): File =
        File(activity.filesDir, "documents").apply { mkdirs() }

    /** Streams a content URI into the private directory and returns the new name. */
    fun copyIntoDocuments(activity: Activity, uri: Uri): String? {
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

    fun displayName(activity: Activity, uri: Uri): String? {
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
    fun safeName(name: String): String? {
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
    fun uniqueDestination(directory: File, name: String): File {
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

    fun mimeTypeFor(name: String): String =
        when (name.substringAfterLast('.', "").lowercase()) {
            "csv" -> "text/csv"
            "tsv", "tab" -> "text/tab-separated-values"
            "dat" -> "application/octet-stream"
            else -> "text/plain"
        }
}
