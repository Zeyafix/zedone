package app.zedone.local

import android.os.Bundle
import androidx.activity.OnBackPressedCallback
import android.webkit.WebView

class MainActivity : TauriActivity() {
  private var activeWebView: WebView? = null

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    activeWebView = webView
    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() {
        webView.evaluateJavascript("""
          (() => {
            const scanner = document.querySelector('#device-scan-cancel');
            if (scanner) { scanner.click(); return true; }
            const close = document.querySelector('[data-workspace-dialog-close], [data-shell-menu-close]');
            if (close) { close.click(); return true; }
            if (location.hash === '#/settings' || /^#\/spaces\/[a-f0-9]{32}$/.test(location.hash)) {
              if (history.length > 1) history.back(); else location.replace('#/spaces');
              return true;
            }
            return false;
          })()
        """.trimIndent()) { consumed ->
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
    activeWebView?.evaluateJavascript("window.dispatchEvent(new Event('zedone-background'))", null)
    super.onStop()
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
  }
}
