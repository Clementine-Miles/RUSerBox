package tech.lolli.toolbox;

import android.content.Context;
import android.content.SharedPreferences;
import android.util.Base64;
import android.util.Log;

import java.nio.charset.StandardCharsets;
import java.security.Key;
import java.security.KeyStore;
import java.security.spec.MGF1ParameterSpec;

import javax.crypto.Cipher;
import javax.crypto.spec.GCMParameterSpec;
import javax.crypto.spec.IvParameterSpec;
import javax.crypto.spec.OAEPParameterSpec;
import javax.crypto.spec.PSource;

/**
 * Reads the store key the Flutter build kept with flutter_secure_storage
 * (10.x, the lollipopkit fork), once, so an update keeps opening `store.db`.
 * Read only: nothing here writes or deletes the old entries.
 *
 * Its layout: the value under `<prefix>_hivePwd` in the `FlutterSecureStorage`
 * preferences is base64(IV + ciphertext); the AES key that encrypts it is
 * stored in `FlutterSecureKeyStorage`, wrapped by an RSA key in the Android
 * Keystore. Two generations: AES-GCM with an RSA-OAEP(SHA-256, MGF1-SHA-1)
 * wrap (alias `<package>.FlutterSecureStoragePluginKeyOAEP`), and the older
 * AES-CBC with RSA-PKCS1 (alias `<package>.FlutterSecureStoragePluginKey`),
 * told apart by the algorithm markers.
 */
final class LegacyVault {
    private static final String TAG = "LegacyVault";
    private static final String DATA_PREFS = "FlutterSecureStorage";
    private static final String KEY_PREFS = "FlutterSecureKeyStorage";
    private static final String PREFIX = "VGhpcyBpcyB0aGUgcHJlZml4IGZvciBhIHNlY3VyZSBzdG9yYWdlCg";
    private static final String GCM_KEY = "AESVGhpcyBpcyB0aGUga2V5IGZvciBhIHNlY3VyZSBzdG9yYWdlIEFFUyBLZXkK";
    private static final String GCM_KEY_V9 = "VGhpcyBpcyB0aGUga2V5IGZvcihBIHNlY3XyZZBzdG9yYWdlIEFFUyBLZXkK";
    private static final String CBC_KEY = "VGhpcyBpcyB0aGUga2V5IGZvciBhIHNlY3VyZSBzdG9yYWdlIEFFUyBLZXkK";
    private static final String MARK_STORAGE = "FlutterSecureSAlgorithmStorage";

    private LegacyVault() {
    }

    /** The 32-byte key, or null when there is none to migrate. */
    static byte[] storeKey(Context ctx) {
        try {
            SharedPreferences data = ctx.getSharedPreferences(DATA_PREFS, Context.MODE_PRIVATE);
            String value = data.getString(PREFIX + "_hivePwd", null);
            if (value == null) return null;
            byte[] blob = Base64.decode(value, Base64.DEFAULT);
            SharedPreferences keys = ctx.getSharedPreferences(KEY_PREFS, Context.MODE_PRIVATE);
            String storage = data.getString(MARK_STORAGE, null);
            byte[] plain = null;
            if (storage == null || storage.contains("GCM")) plain = gcm(ctx, keys, blob);
            if (plain == null) plain = cbc(ctx, keys, blob);
            if (plain == null) return null;
            byte[] key = Base64.decode(new String(plain, StandardCharsets.UTF_8), Base64.URL_SAFE | Base64.NO_WRAP);
            return key.length == 32 ? key : null;
        } catch (Exception e) {
            Log.w(TAG, "no legacy key", e);
            return null;
        }
    }

    private static Key rsa(String alias) throws Exception {
        KeyStore ks = KeyStore.getInstance("AndroidKeyStore");
        ks.load(null);
        return ks.getKey(alias, null);
    }

    private static byte[] gcm(Context ctx, SharedPreferences keys, byte[] blob) {
        try {
            String wrapped = keys.getString(GCM_KEY, keys.getString(GCM_KEY_V9, null));
            if (wrapped == null) return null;
            Key priv = rsa(ctx.getPackageName() + ".FlutterSecureStoragePluginKeyOAEP");
            if (priv == null) return null;
            Cipher unwrap = Cipher.getInstance("RSA/ECB/OAEPPadding", "AndroidKeyStoreBCWorkaround");
            unwrap.init(Cipher.UNWRAP_MODE, priv, new OAEPParameterSpec("SHA-256", "MGF1", MGF1ParameterSpec.SHA1, PSource.PSpecified.DEFAULT));
            Key aes = unwrap.unwrap(Base64.decode(wrapped, Base64.DEFAULT), "AES", Cipher.SECRET_KEY);
            Cipher c = Cipher.getInstance("AES/GCM/NoPadding");
            c.init(Cipher.DECRYPT_MODE, aes, new GCMParameterSpec(128, blob, 0, 12));
            return c.doFinal(blob, 12, blob.length - 12);
        } catch (Exception e) {
            Log.w(TAG, "GCM generation did not open", e);
            return null;
        }
    }

    private static byte[] cbc(Context ctx, SharedPreferences keys, byte[] blob) {
        try {
            String wrapped = keys.getString(CBC_KEY, null);
            if (wrapped == null) return null;
            Key priv = rsa(ctx.getPackageName() + ".FlutterSecureStoragePluginKey");
            if (priv == null) return null;
            Cipher unwrap = Cipher.getInstance("RSA/ECB/PKCS1Padding", "AndroidKeyStoreBCWorkaround");
            unwrap.init(Cipher.UNWRAP_MODE, priv);
            Key aes = unwrap.unwrap(Base64.decode(wrapped, Base64.DEFAULT), "AES", Cipher.SECRET_KEY);
            Cipher c = Cipher.getInstance("AES/CBC/PKCS7Padding");
            c.init(Cipher.DECRYPT_MODE, aes, new IvParameterSpec(blob, 0, 16));
            return c.doFinal(blob, 16, blob.length - 16);
        } catch (Exception e) {
            Log.w(TAG, "CBC generation did not open", e);
            return null;
        }
    }
}
