package app.shurco.tabtool

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.core.content.FileProvider
import java.io.File

/** Clipboard, share sheet and external links: hand-offs to other apps. */
internal object SystemActions {
    fun clipboardGet(activity: Activity): String? {
        return try {
            val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val clip = clipboard.primaryClip ?: return null
            if (clip.itemCount == 0) null else clip.getItemAt(0).text?.toString()
        } catch (error: SecurityException) {
            null
        }
    }

    fun clipboardSet(activity: Activity, text: String) {
        try {
            val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("TabTool", text))
        } catch (error: SecurityException) {
            // Copying is best effort; the app keeps its own value.
        }
    }

    fun shareFile(activity: Activity, path: String): String {
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

    fun openUrl(activity: Activity, url: String): String {
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
}
