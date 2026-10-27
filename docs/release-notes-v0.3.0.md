Port Android nativo Rust/JNI sin audio. Incluye controles táctiles configurables, pausa al perder el foco, récords locales y mejoras del motor y de las paletas contrastadas con la desensamblación NTSC.

Esta versión conserva diferencias conocidas, incluidas temporización PPU/DAS no completamente ciclo-exacta, sincronización del RNG, animaciones de los finales y ausencia de demo. Las capturas visuales son pruebas de regresión del propio port; el emulador x86_64 usa traducción ARM64 y no equivale a verificar un teléfono real. Consulta `docs/fidelity.md` para el detalle.

La firma depende de los secretos del repositorio. Si no se configuró una clave estable, el APK usa la clave de depuración del runner y puede no actualizar instalaciones anteriores; consulta `docs/signing.md` antes de desinstalar.
