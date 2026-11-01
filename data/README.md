# Catálogo incluido

`sky.json` contiene 5.044 estrellas hasta magnitud aparente 6 y 89 grupos de
líneas correspondientes a las 88 constelaciones (Serpens tiene dos partes).
Se genera con `python3 scripts/catalog.py` a partir de estos archivos de
[d3-celestial, de Olaf Frohn](https://github.com/ofrohn/d3-celestial):

Revisión fijada: [`7e720a3de062059d4c5400a379146a601d9010e0`](https://github.com/ofrohn/d3-celestial/tree/7e720a3de062059d4c5400a379146a601d9010e0).

- `data/stars.6.json`: posiciones ecuatoriales J2000 y magnitud, catálogo Hipparcos.
- `data/starnames.json`: nombres y designaciones.
- `data/constellations.lines.json`: figuras de las constelaciones.

La licencia de la distribución fuente está en `LICENSE-d3-celestial` (BSD-3-Clause).
El programa incorpora el JSON en el binario: no descarga datos durante su uso.
La regeneración del catálogo sí necesita Internet. La revisión se actualiza
explícitamente en `scripts/catalog.py` cuando se decide incorporar nuevos datos.

No se incluyen distancias: no se deducen de la magnitud ni se inventan.
