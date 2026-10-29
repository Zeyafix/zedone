// Copyright 2026 ZeDone contributors
// SPDX-License-Identifier: MIT

package app.tauri.plugin

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity
import app.tauri.Logger
import com.fasterxml.jackson.databind.ObjectMapper

/** Main-thread ownership of native plugins; registrations retain no Activity or WebView. */
class PluginLifecycle(
  private val manager: PluginManager,
  private val plugins: MutableMap<String, PluginHandle>
) {
  private data class Registration(
    val type: Class<out Plugin>, val config: String, val mapper: ObjectMapper
  ) {
    // This is the same public constructor contract used by Tauri's Rust registration.
    fun create(activity: AppCompatActivity): Plugin =
      type.getConstructor(Activity::class.java).newInstance(activity)
  }

  private val registrations = linkedMapOf<String, Registration>()
  private val views = linkedMapOf<AppCompatActivity, WebView>()
  private var owner: AppCompatActivity? = null
  private var boundView: WebView? = null

  private fun activityOf(context: Context): AppCompatActivity? = when (context) {
    is AppCompatActivity -> context
    is ContextWrapper -> if (context.baseContext !== context) activityOf(context.baseContext) else null
    else -> null
  }

  private fun release(activity: AppCompatActivity) {
    val previous = plugins.values.toList()
    plugins.clear()
    owner = null
    boundView = null
    for (handle in previous) {
      try { handle.instance.triggerOnDestroy(activity) }
      catch (error: Exception) { Logger.error("Plugin ${handle.name} cleanup failed: ${error.message}") }
    }
  }

  private fun attach(activity: AppCompatActivity, webView: WebView?) {
    check(!activity.isDestroyed) { "Cannot attach plugins to a destroyed Activity" }
    if (owner !== activity) {
      owner?.let { release(it) }
      owner = activity
      for ((name, registration) in registrations) {
        plugins[name] = PluginHandle(manager, name, registration.create(activity), registration.config, registration.mapper)
      }
    }
    if (webView != null) {
      for (handle in plugins.values) {
        if (!handle.loaded || boundView !== webView) handle.load(webView)
      }
      boundView = webView
    }
  }

  fun onActivityAvailable(activity: AppCompatActivity?) {
    if (activity != null && !activity.isDestroyed) attach(activity, views[activity])
  }

  fun onWebViewCreated(current: AppCompatActivity?, webView: WebView) {
    val activity = checkNotNull(activityOf(webView.context)) { "WebView has no Activity owner" }
    if (activity.isDestroyed) return
    views[activity] = webView
    if (activity === current) attach(activity, webView)
  }

  fun register(current: AppCompatActivity?, webView: WebView?, name: String,
               plugin: Plugin, config: String, mapper: ObjectMapper) {
    val registration = Registration(plugin.javaClass, config, mapper)
    registrations[name] = registration
    val viewOwner = webView?.let { activityOf(it.context) }
    if (viewOwner != null && !viewOwner.isDestroyed) views[viewOwner] = webView
    val target = current ?: viewOwner ?: return
    if (target.isDestroyed) return // onCreate will materialize the saved registration.
    if (owner !== target) {
      attach(target, views[target])
      return
    }
    plugins.remove(name)?.instance?.triggerOnDestroy(target)
    val instance = if (webView == null || viewOwner === target) plugin else registration.create(target)
    val handle = PluginHandle(manager, name, instance, config, mapper)
    plugins[name] = handle
    views[target]?.let { handle.load(it); boundView = it }
  }

  fun onDestroy(activity: AppCompatActivity) {
    views.remove(activity)
    if (owner === activity) release(activity)
  }
}
