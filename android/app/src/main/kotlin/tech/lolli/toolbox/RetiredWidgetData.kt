package tech.lolli.toolbox

import android.content.Context
import android.util.Log
import java.security.KeyStore

/**
 * Removes what the retired home-screen widget left on the device: its server
 * list, the read-only Monitor tokens it held (sealed under an Android Keystore
 * key) and that key. The widget itself is gone from the manifest, so nothing
 * reads these any more; leaving them would keep credentials nobody can clear.
 *
 * Idempotent and cheap: deleting a missing preference file or alias is a
 * no-op. The tokens are also revoked on the Monitor agents by the app
 * (`RetiredWidgetTokens` in Dart).
 */
object RetiredWidgetData {
    private const val TAG = "RetiredWidgetData"
    private const val PREFS = "sbm_widget_servers"
    private const val TOKEN_PREFS = "sbm_widget_tokens"
    private const val KEY_ALIAS = "sbm_widget_token_key"

    fun purge(context: Context) {
        try {
            for (name in listOf(PREFS, TOKEN_PREFS)) {
                context.deleteSharedPreferences(name)
            }
        } catch (e: Exception) {
            Log.w(TAG, "Could not delete the widget preferences", e)
        }
        try {
            val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            if (keyStore.containsAlias(KEY_ALIAS)) keyStore.deleteEntry(KEY_ALIAS)
        } catch (e: Exception) {
            Log.w(TAG, "Could not delete the widget token key", e)
        }
    }
}
