package com.jude.tetrisnesrust

import android.content.Context
import android.content.Intent
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/** Exercises the Android preferences -> JNI -> Rust -> preferences round trip. */
@RunWith(AndroidJUnit4::class)
class RecordPersistenceTest {
    private fun record(score: Int, level: Int, name: IntArray): Long {
        var value = score.toLong() or (level.toLong() shl 20)
        name.forEachIndexed { index, char -> value = value or (char.toLong() shl (28 + index * 6)) }
        return value
    }

    private fun handle(activity: MainActivity): Long {
        val field = MainActivity::class.java.getDeclaredField("handle")
        field.isAccessible = true
        return field.getLong(activity)
    }

    @Test fun recordSurvivesActivityRecreation() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        val prefs = context.getSharedPreferences("tetris_scores_v1", Context.MODE_PRIVATE)
        val initial = record(54_321, 12, intArrayOf(1, 2, 3, 27, 36, 43))
        val updated = record(65_432, 14, intArrayOf(24, 25, 26, 30, 38, 43))
        check(prefs.edit().putLong("record_0", initial).commit())
        fun launch(): MainActivity {
            val intent = Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            return instrumentation.startActivitySync(intent) as MainActivity
        }
        val first = launch()
        assertEquals(initial, first.nativeRecord(handle(first), false, 0))
        instrumentation.runOnMainSync {
            first.nativeSetRecord(handle(first), false, 0, updated)
            first.finish()
        }
        instrumentation.waitForIdleSync()
        assertEquals(updated, prefs.getLong("record_0", 0))
        val second = launch()
        assertEquals(updated, second.nativeRecord(handle(second), false, 0))
        instrumentation.runOnMainSync { second.finish() }
        instrumentation.waitForIdleSync()
    }
}
