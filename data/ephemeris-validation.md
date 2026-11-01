# Referencias independientes para efemérides

Los tests comparan Astronomy Engine con tablas de observador de
[NASA/JPL Horizons](https://ssd.jpl.nasa.gov/horizons/), consultadas el 2026-11-01.
Las pruebas usan valores fijados aquí y no hacen solicitudes de red.

Consulta: `https://ssd.jpl.nasa.gov/api/horizons.api` con parámetros:

```text
format=json
COMMAND='10' (Sol), '301' (Luna), '599' (Júpiter)
EPHEM_TYPE='OBSERVER'
CENTER='coord@399'
COORD_TYPE='GEODETIC'
SITE_COORD='0,0,0'
START_TIME='2026-01-01 00:00'
STOP_TIME='2026-01-01 00:01'
STEP_SIZE='1m'
QUANTITIES='4,9,10,20'
CSV_FORMAT='YES'
APPARENT='AIRLESS'
```

Resultados a las 00:00 UTC, observador en el ecuador y meridiano de Greenwich,
altura 0 km; azimut desde el norte hacia el este, sin refracción:

| Cuerpo | Azimut ° | Altura ° | Distancia topocéntrica UA | Fracción iluminada % |
|---|---:|---:|---:|---:|
| Sol | 181.957796 | -66.969505 | 0.98336589953355 | 100.00000 |
| Luna | 309.690817 | 45.154603 | 0.00238302142621 | 91.10401 |
| Júpiter | 28.131672 | 64.886725 | 4.24262710885954 | 99.96878 |

Tolerancias de integración: 0,05° en azimut/altura y 0,1% en distancia. La
comparación lunar detecta la omisión de paralaje topocéntrica; las posiciones
planetarias detectan errores de unidades de AR y de equinoccio.

Las fases se comprueban además cerca de la luna nueva del eclipse solar del
2024-04-08 (18:21 UTC) y de la luna llena del eclipse lunar del 2024-03-25
(07:00 UTC). La iluminación calculada por Astronomy Engine es geocéntrica;
no se exige igualdad exacta con la fracción topocéntrica de Horizons.
