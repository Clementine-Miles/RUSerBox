//! Calls into `MainActivity` (Java) over raw JNI, on the thread that runs
//! `android_main` (attached by android-activity). Only methods of the
//! activity object are called, so no class lookup through a class loader is
//! needed; every call clears a pending Java exception and reports it as
//! `None`/`Err` rather than leaving the VM in an exception state.

use std::ffi::{CStr, CString, c_void};

use jni_sys::{JNI_OK, JNI_VERSION_1_6, JNIEnv, JavaVM, jbyteArray, jint, jobject, jsize, jstring, jvalue};

pub struct Activity {
    vm: *mut JavaVM,
    activity: jobject,
}

macro_rules! call {
    ($env:expr, $ver:tt, $name:ident $(, $a:expr)*) => {
        ((**$env).$ver.$name)($env $(, $a)*)
    };
}

impl Activity {
    /// # Safety
    /// [vm] and [activity] are android-activity's (valid for the process,
    /// the activity a global reference).
    pub unsafe fn new(vm: *mut c_void, activity: *mut c_void) -> Activity {
        Activity { vm: vm.cast(), activity: activity.cast() }
    }

    fn env(&self) -> Option<*mut JNIEnv> {
        let mut env: *mut c_void = std::ptr::null_mut();
        // SAFETY: a valid VM; the calling thread is attached by android-activity
        // (or attached here, and stays attached: it is a long-lived thread).
        unsafe {
            let vm = self.vm;
            let rc = ((**vm).v1_2.GetEnv)(vm, &mut env, JNI_VERSION_1_6);
            if rc != JNI_OK {
                let rc = ((**vm).v1_1.AttachCurrentThread)(vm, &mut env, std::ptr::null_mut());
                if rc != JNI_OK {
                    return None;
                }
            }
        }
        Some(env.cast())
    }

    /// A pending exception, cleared (and described to logcat).
    unsafe fn failed(env: *mut JNIEnv) -> bool {
        unsafe {
            if call!(env, v1_2, ExceptionCheck) {
                call!(env, v1_1, ExceptionDescribe);
                call!(env, v1_1, ExceptionClear);
                return true;
            }
        }
        false
    }

    unsafe fn method(&self, env: *mut JNIEnv, name: &CStr, sig: &CStr) -> Option<jni_sys::jmethodID> {
        unsafe {
            let class = call!(env, v1_1, GetObjectClass, self.activity);
            let id = call!(env, v1_1, GetMethodID, class, name.as_ptr(), sig.as_ptr());
            call!(env, v1_1, DeleteLocalRef, class);
            if Self::failed(env) || id.is_null() { None } else { Some(id) }
        }
    }

    unsafe fn jstring(env: *mut JNIEnv, s: &str) -> jstring {
        let c = CString::new(s.replace('\0', "")).unwrap_or_default();
        unsafe { call!(env, v1_1, NewStringUTF, c.as_ptr()) }
    }

    unsafe fn rstring(env: *mut JNIEnv, s: jstring) -> Option<String> {
        if s.is_null() {
            return None;
        }
        unsafe {
            let p = call!(env, v1_1, GetStringUTFChars, s, std::ptr::null_mut());
            if p.is_null() {
                return None;
            }
            let out = CStr::from_ptr(p).to_string_lossy().into_owned();
            call!(env, v1_1, ReleaseStringUTFChars, s, p);
            Some(out)
        }
    }

    unsafe fn jbytes(env: *mut JNIEnv, b: &[u8]) -> jbyteArray {
        unsafe {
            let a = call!(env, v1_1, NewByteArray, b.len() as jsize);
            call!(env, v1_1, SetByteArrayRegion, a, 0, b.len() as jsize, b.as_ptr().cast());
            a
        }
    }

    unsafe fn rbytes(env: *mut JNIEnv, a: jbyteArray) -> Option<Vec<u8>> {
        if a.is_null() {
            return None;
        }
        unsafe {
            let n = call!(env, v1_1, GetArrayLength, a) as usize;
            let mut v = vec![0u8; n];
            call!(env, v1_1, GetByteArrayRegion, a, 0, n as jsize, v.as_mut_ptr().cast());
            Some(v)
        }
    }

    fn void(&self, name: &CStr, sig: &CStr, args: impl FnOnce(*mut JNIEnv) -> Vec<jvalue>) {
        let Some(env) = self.env() else { return };
        unsafe {
            let Some(m) = self.method(env, name, sig) else { return };
            let a = args(env);
            call!(env, v1_1, CallVoidMethodA, self.activity, m, a.as_ptr());
            Self::failed(env);
            for v in a {
                if !v.l.is_null() {
                    call!(env, v1_1, DeleteLocalRef, v.l);
                }
            }
        }
    }

    fn object(&self, name: &CStr, sig: &CStr, args: impl FnOnce(*mut JNIEnv) -> Vec<jvalue>) -> Option<(*mut JNIEnv, jobject)> {
        let env = self.env()?;
        unsafe {
            let m = self.method(env, name, sig)?;
            let a = args(env);
            let r = call!(env, v1_1, CallObjectMethodA, self.activity, m, a.as_ptr());
            let failed = Self::failed(env);
            for v in a {
                if !v.l.is_null() {
                    call!(env, v1_1, DeleteLocalRef, v.l);
                }
            }
            if failed || r.is_null() { None } else { Some((env, r)) }
        }
    }

    fn string_call(&self, name: &CStr) -> Option<String> {
        let (env, r) = self.object(name, c"()Ljava/lang/String;", |_| vec![])?;
        unsafe {
            let s = Self::rstring(env, r);
            call!(env, v1_1, DeleteLocalRef, r);
            s
        }
    }

    fn bytes_call(&self, name: &CStr, input: Option<&[u8]>) -> Option<Vec<u8>> {
        let sig = if input.is_some() { c"([B)[B" } else { c"()[B" };
        let (env, r) = self.object(name, sig, |env| match input {
            Some(b) => vec![jvalue { l: unsafe { Self::jbytes(env, b) } }],
            None => vec![],
        })?;
        unsafe {
            let v = Self::rbytes(env, r);
            call!(env, v1_1, DeleteLocalRef, r);
            v
        }
    }

    pub fn set_clipboard(&self, text: &str) {
        self.void(c"setClipboard", c"(Ljava/lang/String;)V", |env| vec![jvalue { l: unsafe { Self::jstring(env, text) } }]);
    }

    pub fn clipboard(&self) -> Option<String> {
        self.string_call(c"getClipboard")
    }

    pub fn store_dir(&self) -> Option<String> {
        self.string_call(c"storeDir")
    }

    pub fn locale(&self) -> Option<String> {
        self.string_call(c"localeTag")
    }

    pub fn pick_file(&self, request: i32) {
        self.void(c"pickFile", c"(I)V", |_| vec![jvalue { i: request as jint }]);
    }

    pub fn save_file(&self, name: &str, data: &[u8]) {
        self.void(c"saveFile", c"(Ljava/lang/String;[B)V", |env| unsafe { vec![jvalue { l: Self::jstring(env, name) }, jvalue { l: Self::jbytes(env, data) }] });
    }

    pub fn keep_alive(&self, on: bool) {
        self.void(c"keepAlive", c"(Z)V", |_| vec![jvalue { z: on }]);
    }

    pub fn publish_widgets(&self, payload: &str) {
        self.void(c"publishWidgets", c"(Ljava/lang/String;)V", |env| vec![jvalue { l: unsafe { Self::jstring(env, payload) } }]);
    }

    pub fn seal(&self, plain: &[u8]) -> Option<Vec<u8>> {
        self.bytes_call(c"sealKey", Some(plain))
    }

    pub fn open(&self, sealed: &[u8]) -> Option<Vec<u8>> {
        self.bytes_call(c"openKey", Some(sealed))
    }

    pub fn legacy_key(&self) -> Option<Vec<u8>> {
        self.bytes_call(c"legacyStoreKey", None)
    }

    /// System bar and keyboard insets: left, top, right, bottom (pixels).
    pub fn insets(&self) -> [i32; 4] {
        let Some((env, r)) = self.object(c"insets", c"()[I", |_| vec![]) else { return [0; 4] };
        let mut out = [0i32; 4];
        unsafe {
            if call!(env, v1_1, GetArrayLength, r) >= 4 {
                call!(env, v1_1, GetIntArrayRegion, r, 0, 4, out.as_mut_ptr());
            }
            call!(env, v1_1, DeleteLocalRef, r);
        }
        out
    }

    pub fn is_dark(&self) -> bool {
        let Some(env) = self.env() else { return true };
        unsafe {
            let Some(m) = self.method(env, c"isDarkMode", c"()Z") else { return true };
            let r = call!(env, v1_1, CallBooleanMethodA, self.activity, m, std::ptr::null());
            !Self::failed(env) && r
        }
    }

    /// Sends the app to the background (back at the root page).
    pub fn move_to_back(&self) {
        let Some(env) = self.env() else { return };
        unsafe {
            let Some(m) = self.method(env, c"moveTaskToBack", c"(Z)Z") else { return };
            let a = [jvalue { z: true }];
            call!(env, v1_1, CallBooleanMethodA, self.activity, m, a.as_ptr());
            Self::failed(env);
        }
    }
}

/// Files the activity read for a `pickFile` request, delivered from the UI
/// thread: `MainActivity.nativeFilePicked(int, String, byte[])`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_tech_lolli_toolbox_MainActivity_nativeFilePicked(env: *mut JNIEnv, _this: jobject, request: jint, name: jstring, data: jbyteArray) {
    // SAFETY: arguments come from the JVM for this native method.
    let (name, data) = unsafe { (Activity::rstring(env, name).unwrap_or_default(), Activity::rbytes(env, data).unwrap_or_default()) };
    super::android::file_picked(request, name, data);
}
