package com.example.gameactivityexample

import com.google.androidgamesdk.GameActivity
import android.os.Bundle
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.util.Log

class MainActivity : GameActivity() {
    override fun onCreate(savedInstanceState: android.os.Bundle?) {
        super.onCreate(savedInstanceState)
        checkOverlayPermission()
        checkActivityRecognitionPermission()
    }

    private fun checkActivityRecognitionPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            if (checkSelfPermission(android.Manifest.permission.ACTIVITY_RECOGNITION) != android.content.pm.PackageManager.PERMISSION_GRANTED) {
                requestPermissions(arrayOf(android.Manifest.permission.ACTIVITY_RECOGNITION), 1002)
            }
        }
    }

    private fun checkOverlayPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            if (!Settings.canDrawOverlays(this)) {
                val intent = Intent(
                    Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                    Uri.parse("package:$packageName")
                )
                startActivityForResult(intent, 1001)
            }
        }
    }

    // --- Platform Bridge Example ---
    fun triggerTestEvent(message: String) {
        Log.d("Spot", "收到 Rust 消息: $message")
        // 模拟异步处理后回调 Rust
        sendNativeEvent("test_callback", "来自 Kotlin 的回复: $message")
    }

    override fun onResume() {
        super.onResume()
        window.decorView.requestFocus()
    }

    companion object {
        init {
            System.loadLibrary("spottedcat_android_wrapper")
        }

        @JvmStatic
        external fun sendNativeEvent(type: String, data: String)
    }
}
