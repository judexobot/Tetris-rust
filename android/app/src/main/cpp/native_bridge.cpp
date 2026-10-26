#include <jni.h>
#include <cstdint>
#include <cstddef>

extern "C" {
    void* tr_create();
    void tr_destroy(void* ptr);
    void tr_tick(void* ptr, uint32_t buttons);
    void tr_render(void* ptr, uint32_t* pixels, size_t pixels_count);
    uint32_t tr_score(const void* ptr);
    uint32_t tr_scene(const void* ptr);
    bool tr_game_type_b(const void* ptr);
    void tr_set_best(void* ptr, uint32_t a, uint32_t b);
}

extern "C" JNIEXPORT jlong JNICALL
Java_com_jude_tetrisnesrust_MainActivity_nativeCreate(JNIEnv*, jobject) {
    return static_cast<jlong>(reinterpret_cast<intptr_t>(tr_create()));
}
extern "C" JNIEXPORT void JNICALL
Java_com_jude_tetrisnesrust_MainActivity_nativeDestroy(JNIEnv*, jobject, jlong handle) {
    tr_destroy(reinterpret_cast<void*>(static_cast<intptr_t>(handle)));
}
extern "C" JNIEXPORT void JNICALL
Java_com_jude_tetrisnesrust_MainActivity_nativeTick(JNIEnv*, jobject, jlong handle, jint buttons) {
    tr_tick(reinterpret_cast<void*>(static_cast<intptr_t>(handle)), static_cast<uint32_t>(buttons));
}
extern "C" JNIEXPORT void JNICALL
Java_com_jude_tetrisnesrust_MainActivity_nativeRender(JNIEnv* env, jobject, jlong handle, jintArray array) {
    if (!array || env->GetArrayLength(array) < 256*240) return;
    jint* data=env->GetIntArrayElements(array, nullptr);
    if (!data) return;
    tr_render(reinterpret_cast<void*>(static_cast<intptr_t>(handle)),
              reinterpret_cast<uint32_t*>(data), 256*240);
    env->ReleaseIntArrayElements(array, data, 0);
}

extern "C" JNIEXPORT jint JNICALL
Java_com_jude_tetrisnesrust_MainActivity_nativeScore(JNIEnv*, jobject, jlong handle) {
    return static_cast<jint>(tr_score(reinterpret_cast<void*>(static_cast<intptr_t>(handle))));
}
extern "C" JNIEXPORT jint JNICALL
Java_com_jude_tetrisnesrust_MainActivity_nativeScene(JNIEnv*, jobject, jlong handle) {
    return static_cast<jint>(tr_scene(reinterpret_cast<void*>(static_cast<intptr_t>(handle))));
}
extern "C" JNIEXPORT jboolean JNICALL
Java_com_jude_tetrisnesrust_MainActivity_nativeGameTypeB(JNIEnv*, jobject, jlong handle) {
    return tr_game_type_b(reinterpret_cast<void*>(static_cast<intptr_t>(handle))) ? JNI_TRUE : JNI_FALSE;
}
extern "C" JNIEXPORT void JNICALL
Java_com_jude_tetrisnesrust_MainActivity_nativeSetBest(JNIEnv*, jobject, jlong handle, jint a, jint b) {
    tr_set_best(reinterpret_cast<void*>(static_cast<intptr_t>(handle)),static_cast<uint32_t>(a),static_cast<uint32_t>(b));
}
