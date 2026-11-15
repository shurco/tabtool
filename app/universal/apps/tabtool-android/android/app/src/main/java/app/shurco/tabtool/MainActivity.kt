package app.shurco.tabtool

import android.content.Intent
import android.content.res.Configuration
import android.net.Uri
import android.os.Bundle
import android.view.View
import androidx.activity.OnBackPressedCallback
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import com.google.androidgamesdk.GameActivity

class MainActivity : GameActivity() {
    private val editorBackCallback = object : OnBackPressedCallback(true) {
        override fun handleOnBackPressed() {
            TabToolBridge.handleBack()
        }
    }

    /** The VIEW/SEND intent already copied by this activity, if any. */
    private var handledIntentKey: String? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        TabToolBridge.attach(this)
        onBackPressedDispatcher.addCallback(this, editorBackCallback)
        handledIntentKey = savedInstanceState?.getString(STATE_HANDLED_INTENT)
        applySystemBarAppearance()
        val key = importIntentKey(intent)
        val fromHistory = intent.flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY != 0
        // A restored process already has its document in app-state.json. Only
        // a genuinely different VIEW/SEND is imported; the saved key marks the
        // intent that was already consumed before the recreation.
        if (key != null && !fromHistory && key != handledIntentKey) {
            TabToolBridge.handleIntent(intent)
            handledIntentKey = key
        }
        window.decorView.post { window.decorView.requestApplyInsets() }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        importIntentKey(intent)?.let {
            TabToolBridge.handleIntent(intent)
            handledIntentKey = it
        }
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        handledIntentKey?.let { outState.putString(STATE_HANDLED_INTENT, it) }
    }

    override fun onStart() {
        super.onStart()
        TabToolBridge.updateForeground(true)
    }

    override fun onPause() {
        TabToolBridge.updateForeground(false)
        super.onPause()
    }

    override fun onApplyWindowInsets(view: View, insets: WindowInsetsCompat): WindowInsetsCompat {
        TabToolBridge.updateInsets(insets)
        return super.onApplyWindowInsets(view, insets)
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        TabToolBridge.cancelTextEditor()
        applySystemBarAppearance()
        super.onConfigurationChanged(newConfig)
    }

    /** Light mode needs dark status and navigation bar icons. */
    private fun applySystemBarAppearance() {
        val controller = WindowCompat.getInsetsController(window, window.decorView)
        val light = (resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) !=
            Configuration.UI_MODE_NIGHT_YES
        controller.isAppearanceLightStatusBars = light
        controller.isAppearanceLightNavigationBars = light
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        TabToolBridge.onActivityResult(requestCode, resultCode, data)
    }

    override fun onDestroy() {
        TabToolBridge.detach()
        super.onDestroy()
    }

    /** A stable key for the intent a copy consumes, or null for a launcher. */
    private fun importIntentKey(intent: Intent?): String? {
        if (intent == null) return null
        val action = intent.action
        if (action != Intent.ACTION_VIEW && action != Intent.ACTION_SEND) return null
        val stream =
            if (action == Intent.ACTION_SEND) {
                @Suppress("DEPRECATION")
                intent.getParcelableExtra(Intent.EXTRA_STREAM) as? Uri
            } else {
                null
            }
        return listOf(action, intent.dataString, intent.type, stream?.toString()).joinToString("|")
    }

    private companion object {
        const val STATE_HANDLED_INTENT = "app.shurco.tabtool.handledIntent"
    }
}
