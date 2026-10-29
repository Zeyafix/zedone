package app.zedone.prototype

import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.activity.OnBackPressedCallback
import android.webkit.WebView

class MainActivity : TauriActivity() {
  private var prototypeWebView: WebView? = null

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    prototypeWebView = webView
    // Consume Back only while the scan UI is active; keep Wry's navigation otherwise.
    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() {
        webView.evaluateJavascript("(() => { const button = document.querySelector('#scan-cancel'); if (!button) return false; button.click(); return true; })()") { consumed ->
          if (consumed != "true") {
            isEnabled = false
            onBackPressedDispatcher.onBackPressed()
            isEnabled = true
          }
        }
      }
    })
  }

  override fun onStop() {
    prototypeWebView?.evaluateJavascript("window.dispatchEvent(new Event('prototype-background'))", null)
    super.onStop()
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }
}
