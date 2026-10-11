package com.scarpify.app

import android.app.*
import android.content.*
import android.content.pm.ServiceInfo
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.media.MediaMetadata
import android.media.session.MediaSession
import android.media.session.PlaybackState
import android.os.*
import app.tauri.plugin.Channel

/** Foreground media service: keeps the native Rust player alive with the screen off.
 * It owns audio focus and the OS media session; audio itself remains in rodio/AAudio. */
class PlaybackService: Service() {
    private lateinit var session: MediaSession
    private val noisy = object: BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == AudioManager.ACTION_AUDIO_BECOMING_NOISY) control("pause")
        }
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        getSystemService(NotificationManager::class.java).createNotificationChannel(
            NotificationChannel(CHANNEL, "Music playback", NotificationManager.IMPORTANCE_LOW))
        session = MediaSession(this, "SCARPIFY")
        session.setCallback(object: MediaSession.Callback() {
            override fun onPlay() { try { resume(this@PlaybackService); control("resume") } catch (_: Exception) {} }
            override fun onPause() { control("pause") }
            override fun onStop() { control("stop"); stopSelf() }
            override fun onSkipToNext() { control("next") }
            override fun onSkipToPrevious() { control("previous") }
        })
        session.isActive = true
        val filter = IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY)
        if (Build.VERSION.SDK_INT >= 33) registerReceiver(noisy, filter, RECEIVER_NOT_EXPORTED)
        else @Suppress("DEPRECATION") registerReceiver(noisy, filter)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            "pause" -> control("pause")
            "resume" -> try { resume(this); control("resume") } catch (_: Exception) {}
            "next", "previous" -> control(intent.action!!)
            "stop" -> { control("stop"); stopSelf(); return START_NOT_STICKY }
        }
        refresh()
        // Never restart into phantom playback after the process was killed.
        return START_NOT_STICKY
    }

    private fun action(label: String, command: String, icon: Int): Notification.Action {
        val intent = Intent(this, PlaybackService::class.java).setAction(command)
        val pending = PendingIntent.getService(this, command.hashCode(), intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        return Notification.Action.Builder(icon, label, pending).build()
    }

    private fun refresh() {
        val open = packageManager.getLaunchIntentForPackage(packageName)
        val pending = PendingIntent.getActivity(this, 0, open, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        val playing = kind == "playing" || kind == "progress"
        session.setMetadata(MediaMetadata.Builder().putString(MediaMetadata.METADATA_KEY_TITLE, title)
            .putString(MediaMetadata.METADATA_KEY_ARTIST, artist).putLong(MediaMetadata.METADATA_KEY_DURATION, duration).build())
        session.setPlaybackState(PlaybackState.Builder()
            .setActions(PlaybackState.ACTION_PLAY or PlaybackState.ACTION_PAUSE or PlaybackState.ACTION_STOP or PlaybackState.ACTION_SKIP_TO_NEXT or PlaybackState.ACTION_SKIP_TO_PREVIOUS)
            .setState(if (playing) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED, position, if (playing) 1f else 0f).build())
        val notification = Notification.Builder(this, CHANNEL)
            .setSmallIcon(android.R.drawable.ic_media_play).setContentTitle(title).setContentText(artist)
            .setContentIntent(pending).setOnlyAlertOnce(true).setOngoing(playing)
            .addAction(action("Previous", "previous", android.R.drawable.ic_media_previous))
            .addAction(if (playing) action("Pause", "pause", android.R.drawable.ic_media_pause) else action("Play", "resume", android.R.drawable.ic_media_play))
            .addAction(action("Next", "next", android.R.drawable.ic_media_next))
            .addAction(action("Stop", "stop", android.R.drawable.ic_menu_close_clear_cancel))
            .setStyle(Notification.MediaStyle().setMediaSession(session.sessionToken).setShowActionsInCompactView(0, 1, 2)).build()
        if (Build.VERSION.SDK_INT >= 29) startForeground(NOTIFICATION, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
        else startForeground(NOTIFICATION, notification)
    }

    override fun onBind(intent: Intent?) = null
    override fun onTaskRemoved(rootIntent: Intent?) { control("stop"); stopSelf() }
    override fun onDestroy() {
        instance = null
        if (wakeLock?.isHeld == true) wakeLock?.release()
        main.removeCallbacks(expirePause)
        control("stop")
        unregisterReceiver(noisy)
        session.release()
        focus?.let { audioManager?.abandonAudioFocusRequest(it) }
        focus = null
        instance = null
        super.onDestroy()
    }

    companion object {
        private const val CHANNEL = "scarpify-playback"
        private const val NOTIFICATION = 41
        var channel: Channel? = null
        private var instance: PlaybackService? = null
        private var audioManager: AudioManager? = null
        private var focus: AudioFocusRequest? = null
        private var title = "SCARPIFY"
        private var artist = ""
        private var duration = 0L
        private var position = 0L
        private var kind = "paused"
        private var lastWakeRefresh = 0L
        private var resumeAfterFocus = false
        private val main = Handler(Looper.getMainLooper())
        private var wakeLock: PowerManager.WakeLock? = null
        private val expirePause = Runnable { if (kind == "paused") instance?.stopSelf() }

        fun control(action: String) {
            if (action == "pause" || action == "stop") resumeAfterFocus = false
            channel?.sendObject(action)
        }

        fun prepare(context: Context, trackTitle: String, trackArtist: String, durationMs: Long) {
            title = trackTitle; artist = trackArtist; duration = durationMs; position = 0
            resume(context)
        }

        fun resume(context: Context) {
            audioManager = context.applicationContext.getSystemService(AudioManager::class.java)
            if (focus == null) focus = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build())
                .setOnAudioFocusChangeListener({ change ->
                    when (change) {
                        AudioManager.AUDIOFOCUS_LOSS -> control("pause")
                        AudioManager.AUDIOFOCUS_LOSS_TRANSIENT, AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> {
                            val wasPlaying = kind == "playing" || kind == "progress"
                            control("pause"); resumeAfterFocus = wasPlaying
                        }
                        AudioManager.AUDIOFOCUS_GAIN -> if (resumeAfterFocus) { resumeAfterFocus = false; control("resume") }
                    }
                }, main).build()
            if (audioManager!!.requestAudioFocus(focus!!) != AudioManager.AUDIOFOCUS_REQUEST_GRANTED)
                throw IllegalStateException("Another application owns audio focus")
            main.removeCallbacks(expirePause)
            try {
                context.startForegroundService(Intent(context, PlaybackService::class.java))
            } catch (error: RuntimeException) {
                audioManager?.abandonAudioFocusRequest(focus!!)
                resumeAfterFocus = false
                if (wakeLock?.isHeld == true) wakeLock?.release()
                throw error
            }
            if (wakeLock == null) wakeLock = context.applicationContext.getSystemService(PowerManager::class.java)
                .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "scarpify:playback").apply { setReferenceCounted(false) }
            wakeLock?.acquire(10 * 60 * 1000L)
        }

        fun update(state: String, positionMs: Long) {
            main.post {
                if (state == "progress") {
                    position = positionMs
                    if (SystemClock.elapsedRealtime() - lastWakeRefresh > 60_000) {
                        wakeLock?.acquire(10 * 60 * 1000L)
                        lastWakeRefresh = SystemClock.elapsedRealtime()
                    }
                    return@post
                }
                if (state == "stopped") { instance?.stopSelf(); return@post }
                kind = state; position = positionMs
                if (state == "playing") {
                    main.removeCallbacks(expirePause)
                    wakeLock?.acquire(10 * 60 * 1000L)
                } else if (state == "paused" || state == "ended" || state == "error") {
                    if (wakeLock?.isHeld == true) wakeLock?.release()
                    if (state != "paused") kind = "paused"
                    main.removeCallbacks(expirePause)
                    main.postDelayed(expirePause, 10 * 60 * 1000L)
                }
                instance?.refresh()
            }
        }
    }
}
