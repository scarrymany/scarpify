package com.scarpify.app

import android.app.Activity
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class ConfigureArgs { lateinit var channel: Channel }
@InvokeArg
class PrepareArgs { var title: String = ""; var artist: String = ""; var durationMs: Long? = null }
@InvokeArg
class UpdateArgs { var kind: String = ""; var positionMs: Long = 0 }

@TauriPlugin
class PlaybackPlugin(private val activity: Activity): Plugin(activity) {
    override fun load(webView: WebView) {
        (activity as AppCompatActivity).onBackPressedDispatcher.addCallback(activity,
            object: OnBackPressedCallback(true) {
                override fun handleOnBackPressed() {
                    if (PlaybackService.channel == null) activity.moveTaskToBack(true)
                    else PlaybackService.control("back")
                }
            })
    }

    @Command
    fun configure(invoke: Invoke) {
        PlaybackService.channel = invoke.parseArgs(ConfigureArgs::class.java).channel
        invoke.resolve()
    }

    @Command
    fun prepare(invoke: Invoke) {
        val args = invoke.parseArgs(PrepareArgs::class.java)
        try {
            PlaybackService.prepare(activity, args.title, args.artist, args.durationMs ?: 0)
            invoke.resolve()
        } catch (error: Exception) { invoke.reject(error.message ?: "Unable to start Android media playback") }
    }

    @Command
    fun update(invoke: Invoke) {
        val args = invoke.parseArgs(UpdateArgs::class.java)
        PlaybackService.update(args.kind, args.positionMs)
        invoke.resolve()
    }

    @Command
    fun resume(invoke: Invoke) {
        try {
            PlaybackService.resume(activity)
            invoke.resolve()
        } catch (error: Exception) { invoke.reject(error.message ?: "Audio focus unavailable") }
    }

    @Command
    fun background(invoke: Invoke) { activity.moveTaskToBack(true); invoke.resolve() }
}
