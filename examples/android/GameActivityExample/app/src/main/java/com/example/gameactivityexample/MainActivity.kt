package com.example.gameactivityexample

import com.google.androidgamesdk.GameActivity
import android.os.Bundle
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.util.Log
import androidx.health.connect.client.HealthConnectClient
import androidx.health.connect.client.PermissionController
import androidx.health.connect.client.permission.HealthPermission
import androidx.health.connect.client.records.StepsRecord
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

class MainActivity : GameActivity() {
    private val tag = "SpotExample"
    private val activityScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private val healthPermissions = setOf(HealthPermission.getReadPermission(StepsRecord::class))
    private var didRequestHealthPermissions = false

    private val requestHealthPermissions =
        registerForActivityResult(PermissionController.createRequestPermissionResultContract()) { granted ->
            if (granted.containsAll(healthPermissions)) {
                Log.d(tag, "Health Connect READ_STEPS granted")
                sendNativeEvent("health_connect_permission", "granted")
                sendNativeEvent("health_connect_status", "Health Connect permission granted")
            } else {
                Log.w(tag, "Health Connect READ_STEPS denied")
                sendNativeEvent("health_connect_permission", "denied")
                sendNativeEvent("health_connect_status", "Health Connect permission denied")
            }
        }

    override fun onCreate(savedInstanceState: android.os.Bundle?) {
        super.onCreate(savedInstanceState)
        checkOverlayPermission()
        checkActivityRecognitionPermission()
    }

    private fun checkActivityRecognitionPermission() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) {
            Log.d(tag, "ACTIVITY_RECOGNITION runtime permission not required below Android 10")
            return
        }

        if (checkSelfPermission(android.Manifest.permission.ACTIVITY_RECOGNITION) == android.content.pm.PackageManager.PERMISSION_GRANTED) {
            Log.d(tag, "ACTIVITY_RECOGNITION granted; today's step count can be read")
            return
        }

        Log.d(tag, "Requesting ACTIVITY_RECOGNITION for today's step count demo")
        requestPermissions(arrayOf(android.Manifest.permission.ACTIVITY_RECOGNITION), REQUEST_ACTIVITY_RECOGNITION)
    }

    private fun checkOverlayPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            if (!Settings.canDrawOverlays(this)) {
                val intent = Intent(
                    Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                    Uri.parse("package:$packageName")
                )
                startActivityForResult(intent, REQUEST_OVERLAY_PERMISSION)
            }
        }
    }

    // --- Platform Bridge Example ---
    fun triggerTestEvent(message: String) {
        Log.d("Spot", "收到 Rust 消息: $message")
        // 模拟异步处理后回调 Rust
        sendNativeEvent("test_callback", "来自 Kotlin 的回复: $message")
    }

    fun requestHealthConnectPermissionFromRust() {
        Log.d(tag, "Rust requested Health Connect permission")
        ensureHealthConnectPermission()
    }

    override fun onResume() {
        super.onResume()
        window.decorView.requestFocus()
        checkActivityRecognitionPermission()
    }

    override fun onDestroy() {
        super.onDestroy()
        activityScope.cancel()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)

        if (requestCode != REQUEST_ACTIVITY_RECOGNITION) {
            return
        }

        val granted =
            grantResults.isNotEmpty() &&
                grantResults[0] == android.content.pm.PackageManager.PERMISSION_GRANTED
        if (granted) {
            Log.d(tag, "ACTIVITY_RECOGNITION granted")
            Log.d(tag, "Recreating activity so native sensors are registered after permission grant")
            recreate()
        } else {
            Log.w(tag, "ACTIVITY_RECOGNITION denied; today's step count will stay unavailable")
        }
    }

    private fun ensureHealthConnectPermission() {
        when (HealthConnectClient.getSdkStatus(this)) {
            HealthConnectClient.SDK_UNAVAILABLE -> {
                sendNativeEvent("health_connect_permission", "sdk_unavailable")
                sendNativeEvent("health_connect_status", "Health Connect unavailable on this device")
            }
            HealthConnectClient.SDK_UNAVAILABLE_PROVIDER_UPDATE_REQUIRED -> {
                sendNativeEvent("health_connect_permission", "provider_update_required")
                sendNativeEvent(
                    "health_connect_status",
                    "Install or update Health Connect to show 7-day history"
                )
            }
            HealthConnectClient.SDK_AVAILABLE -> {
                val client = try {
                    HealthConnectClient.getOrCreate(this)
                } catch (e: Exception) {
                    Log.e(tag, "Unable to initialize Health Connect", e)
                    sendNativeEvent("health_connect_permission", "init_failed")
                    sendNativeEvent(
                        "health_connect_status",
                        "Failed to initialize Health Connect"
                    )
                    return
                }

                activityScope.launch {
                    try {
                        val granted = client.permissionController.getGrantedPermissions()
                        if (granted.containsAll(healthPermissions)) {
                            sendNativeEvent("health_connect_permission", "granted")
                            sendNativeEvent("health_connect_status", "Health Connect permission granted")
                        } else if (!didRequestHealthPermissions) {
                            didRequestHealthPermissions = true
                            sendNativeEvent("health_connect_permission", "requesting")
                            sendNativeEvent(
                                "health_connect_status",
                                "Requesting Health Connect permission"
                            )
                            requestHealthPermissions.launch(healthPermissions)
                        } else {
                            sendNativeEvent("health_connect_permission", "denied")
                            sendNativeEvent(
                                "health_connect_status",
                                "Health Connect permission required for 7-day history"
                            )
                        }
                    } catch (e: Exception) {
                        Log.e(tag, "Failed to check Health Connect permissions", e)
                        sendNativeEvent("health_connect_permission", "check_failed")
                        sendNativeEvent(
                            "health_connect_status",
                            "Failed to check Health Connect permissions"
                        )
                    }
                }
            }
        }
    }

    companion object {
        private const val REQUEST_OVERLAY_PERMISSION = 1001
        private const val REQUEST_ACTIVITY_RECOGNITION = 1002

        init {
            System.loadLibrary("spottedcat_android_wrapper")
        }

        @JvmStatic
        external fun sendNativeEvent(type: String, data: String)
    }
}
