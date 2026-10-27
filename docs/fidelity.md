# Referencia y límites de fidelidad

Referencia inspeccionada: [CelestialAmber/TetrisNESDisasm](https://github.com/CelestialAmber/TetrisNESDisasm), commit `94581c0306fa0009fe11fde48222875549d15b8b`, `main.asm`, `constants.asm`, `gfx/` y `gfx/nametables/`. El port es una implementación nativa, no ejecuta el ensamblador.

## Contrastado y cubierto

- `orientationTable`, `rotationTable` y `spawnTable`: siete piezas, 19 orientaciones, sin wall kicks. La prueba incluye paredes y bloques.
- `framesPerDropTable`: NTSC 0–28 y un fotograma por fila a partir de 29. La prueba incluye valores superiores a 29.
- `shift_tetrimino` y `drop_tetrimino`: DAS inicial 16, repetición 6, carga al chocar, prioridad de derecha y caída cada dos fotogramas tras la pulsación; los puntos por caída se descartan al soltar abajo.
- `generateNextPseudorandomNumber` y `pickRandomTetrimino`: LFSR de 16 bits, avance por fotograma y reroll con el ID de orientación anterior. Hay vectores deterministas, pero véanse los límites de sincronización más abajo.
- `initPlayfieldForTypeB`: doce filas, tabla de bloques, hueco por rechazo y borrado para las seis alturas. Objetivo 25 líneas y bonus por dígito de nivel y altura.
- `updateLinesAndStatistics`: puntuación, saturación a 999999 y comparación de decenas BCD con el nivel, incluida la transición anterior al cálculo de puntos. Las pruebas cubren las transiciones de nivel 15 y 19.
- Paleta por nivel de `colorTable`, gráficos y nametables originales, seis caracteres y tres puestos por modo; los empates favorecen la entrada nueva.

## Diferencias conocidas

- La cadencia de entrada tras el bloqueo es una aproximación por altura: no se reproduce el calendario exacto de transferencia de filas PPU, ni todas las ventanas de DAS/rotación y de colisión durante la aparición. La secuencia de borrado representa cinco pares desde el centro, sin reproducir cada etapa de PPU y sondeo de entrada.
- El LFSR y la elección de pieza proceden del original, pero las esperas de inicialización y algunas llamadas adicionales al RNG no son ciclo-exactas; dos partidas con el mismo tiempo de entrada no tienen garantizada la misma secuencia que una NES. El acceso 6502 fuera del campo no se emula.
- El modo B usa los fondos finales originales y cuenta los bonos con temporización; los sprites y animaciones de los personajes no están implementados. El modo A muestra los cinco cohetes originales según la puntuación con despegue aproximado; faltan parches del escenario y sincronización exacta de sprites. La cortina de derrota es una aproximación basada en el intervalo de cuatro fotogramas, sin las esperas y la secuencia completa de NES.
- Menús y cursores son aproximaciones de los originales; no hay pantalla legal, demo, menú de música ni todos los parches gráficos de modo B. El juego no reproduce sonido por decisión de diseño.
- Las estadísticas por pieza y la puntuación se representan de forma convencional; no se reproducen todos los desbordamientos, corrupción de memoria o gráficos de los niveles y contadores extremos. La tabla de récords usa almacenamiento Android, no SRAM NES; sólo se migra el mejor valor simple de las versiones anteriores.
- El bucle usa tiempo fijo de 60 Hz y limita la recuperación tras bloqueos largos del hilo de interfaz; una prueba de humo no mide fluidez sostenida ni latencia táctil. Los mandos analógicos y multitáctiles requieren todavía pruebas físicas en más dispositivos.
- Las capturas de `tests/visual` son patrones de regresión generados por este port, no referencias de un hardware NES. No hay comparación de fotogramas con una captura de hardware ni prueba en el Samsung Galaxy A15 SM-A155M.

## Recursos y regeneración

`core/assets/*.raw` contiene 256 tiles de 8 × 8 con índices de color; `*.map` contiene 960 celdas de nametable y 64 bytes de atributos. Las imágenes `game`, `title_menu`, `typeA_ending` y `typeB_ending`, y los mapas de menús, partida, nombres y finales, se extraen de `gfx/` de la revisión citada. Para regenerarlos con Pillow instalado, crea un ZIP de esa revisión con una carpeta raíz y ejecuta `python3 tools/extract_assets.py archivo.zip`. Los recursos son propiedad de sus respectivos titulares.
