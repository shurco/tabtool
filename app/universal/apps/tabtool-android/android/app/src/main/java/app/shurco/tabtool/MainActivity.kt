package app.shurco.tabtool

import android.content.Intent
import android.os.Bundle
import android.view.View
import androidx.core.view.WindowInsetsCompat
import com.google.androidgamesdk.GameActivity

class MainActivity : GameActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        TabToolBridge.attach(this)
        window.decorView.post { window.decorView.requestApplyInsets() }
    }

    override fun onApplyWindowInsets(view: View, insets: WindowInsetsCompat): WindowInsetsCompat {
        TabToolBridge.updateInsets(insets)
        return super.onApplyWindowInsets(view, insets)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        TabToolBridge.onActivityResult(requestCode, resultCode, data)
    }
}
