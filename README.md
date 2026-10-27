# Tetris NES Rust — Android

Port nativo de Tetris NES NTSC de 1989. La lógica y el renderizador de 256 × 240 están escritos en Rust; Android usa Kotlin para el ciclo de vida, la entrada, las preferencias y la presentación, y una capa C++ JNI para llamar al motor. No utiliza emulación de la CPU NES, WebView, red, publicidad ni audio. Identificador de paquete: `com.jude.tetrisnesrust`.

## Jugar

En la pantalla inicial pulsa START. En la selección de juego usa izquierda/derecha para A o B y START para continuar. En la cuadrícula de nivel se usa el D-pad; en B, A cambia entre selección de nivel y altura. Mantén A y pulsa START para elegir los niveles ocultos 10–19. En la partida, A y B giran, abajo acelera la caída, SELECT activa/desactiva la vista previa y START pausa. La pérdida de foco también pausa; para seguir, vuelve a pulsar START.

La rueda de la esquina abre los ajustes táctiles. Se pueden cambiar tamaño y opacidad, y «Mover botones» permite arrastrar cada botón; confirma con «OK». Se guardan posiciones distintas en vertical y horizontal. También se admiten botones de mandos Bluetooth/USB y ejes de joystick compatibles con Android. Los tres récords, nombres de seis caracteres y niveles de cada modo se guardan localmente. En la pantalla de nombres, arriba/abajo cambia el carácter, A/derecha y B/izquierda mueven el cursor y START confirma.

## Compilar y comprobar

`cargo test --workspace --locked` ejecuta las pruebas del motor. `python3 tools/visual_check.py` genera capturas lógicas PNG y las compara con los patrones del repositorio. Estos patrones detectan cambios en el port; no son capturas de una NES ni demuestran paridad visual.

Para Android hacen falta JDK 17, Gradle 8.9, SDK 35, NDK 27.2.12479018, CMake 3.22.1, Rust 1.99.1 con `aarch64-linux-android` y `cargo-ndk` 4.1.2:

```sh
cargo install cargo-ndk --version 4.1.2 --locked
cargo ndk -t arm64-v8a build -p tetris_nes_core --release --locked
mkdir -p android/app/src/main/cpp/prebuilt/arm64-v8a
cp target/aarch64-linux-android/release/libtetris_nes_core.a android/app/src/main/cpp/prebuilt/arm64-v8a/
gradle -p android :app:assembleRelease --no-daemon
tools/tests/verify_apk.sh android/app/build/outputs/apk/release/app-release.apk
tools/tests/android_smoke.sh android/app/build/outputs/apk/release/app-release.apk target/android-smoke
```

La última prueba instala y elimina el paquete existente: úsala únicamente en un emulador desechable. Comprueba actividad, JNI y supervivencia durante una sesión básica, y guarda capturas y Logcat. La imagen `google_apis` x86_64 de Android 35 admite ARM64 mediante `libndk_translation`; esa prueba usa el APK ARM64 real, pero no sustituye una prueba en un teléfono ARM64.

`gradle -p android :app:assembleReleaseAndroidTest` compila la prueba instrumentada de persistencia. Después de instalar el APK principal en un emulador, `tools/tests/android_persistence.sh android/app/build/outputs/apk/androidTest/release/app-release-androidTest.apk target/android-persistence` verifica que una entrada modificada sobreviva a la recreación de la actividad y se restaure a Rust mediante JNI. La prueba modifica las preferencias del emulador.

El workflow ejecuta las pruebas Rust y visuales, compila ARM64, inspecciona firma, manifiesto, DEX, símbolos JNI y alineación para páginas de 16 KB, e instala y ejecuta el APK en un emulador. Sólo sube el APK después de pasar esas pruebas. El tag `v0.3.0` crea la Release únicamente tras el éxito del job. Véanse [fidelidad y pruebas](docs/fidelity.md) y [firma](docs/signing.md).

## Estado y recursos

Hay diferencias pendientes respecto al original: temporización exacta de entrada y de PPU, secuencias y personajes de finales, selección y presentación de pantallas, casos límite de niveles/estadísticas altos y sincronización completa del RNG durante todas las transiciones. El audio permanece desactivado. Ninguna prueba de este repositorio demuestra funcionamiento en un Galaxy A15 real. La lista detallada está en [docs/fidelity.md](docs/fidelity.md).

Las imágenes de tiles y nametables se derivan de [TetrisNESDisasm](https://github.com/CelestialAmber/TetrisNESDisasm), revisión `94581c0306fa0009fe11fde48222875549d15b8b`, mediante `tools/extract_assets.py`; son recursos de Nintendo y no se relicencian por este proyecto. La licencia MIT declarada en el paquete Rust se refiere al código nuevo.
