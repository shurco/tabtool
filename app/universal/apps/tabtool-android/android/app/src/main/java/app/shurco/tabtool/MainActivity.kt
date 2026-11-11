package app.shurco.tabtool

import android.content.Intent
import android.content.res.Configuration
import android.os.Bundle
import android.view.View
import androidx.activity.OnBackPressedCallback
import androidx.core.view.WindowInsetsCompat
import com.google.androidgamesdk.GameActivity

class MainActivity : GameActivity() {
    private val editorBackCallback = object : OnBackPressedCallback(true) {
        override fun handleOnBackPressed() {
            if (TabToolBridge.cancelTextEditor()) {
                return
            }
            isEnabled = false
            onBackPressedDispatcher.onBackPressed()
            isEnabled = true
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        TabToolBridge.attach(this)
        onBackPressedDispatcher.addCallback(this, editorBackCallback)
        window.decorView.post { window.decorView.requestApplyInsets() }
    }

    override fun onApplyWindowInsets(view: View, insets: WindowInsetsCompat): WindowInsetsCompat {
        TabToolBridge.updateInsets(insets)
        return super.onApplyWindowInsets(view, insets)
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        TabToolBridge.cancelTextEditor()
        super.onConfigurationChanged(newConfig)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        TabToolBridge.onActivityResult(requestCode, resultCode, data)
    }

    override fun onDestroy() {
        TabToolBridge.detach()
        super.onDestroy()
    }
}
