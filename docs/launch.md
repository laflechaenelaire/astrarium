# Launch post

Publish the repository first, then attach `assets/preview.png` to the post.
The suggested mention is **@dhh** (David Heinemeier Hansson).

## English

> Built Astrarium on Omarchy: an offline planetarium in your terminal. ✦
>
> 5,000+ stars, planets, Sun & Moon. Drag to explore, scroll to zoom. Rust, keyboard + mouse, English/Spanish.
>
> @dhh — a little sky for the terminal.
>
> https://github.com/laflechaenelaire/astrarium

## Español

> Hice Astrarium en Omarchy: un planetario offline en la terminal. ✦
>
> Más de 5.000 estrellas, planetas, Sol y Luna. Arrastrás para explorar, zoom con la rueda. Rust, inglés/español.
>
> @dhh
>
> https://github.com/laflechaenelaire/astrarium

Both drafts fit the usual 280-character limit when the link is counted as a
23-character t.co URL. The app is an independent project built on Omarchy;
neither the post nor the README implies official affiliation.

## Capture recipe

The screenshot uses a real Alacritty window, a fixed observer and a paused sky:

```sh
astrarium --lang en --lat=-34.6037 --lon=-58.3816 \
  --location 'Buenos Aires' --at=2026-01-01T03:00:00Z
```

Use a wide terminal, then press Tab to select the Moon without moving the view.
The included capture uses a 2000×1100 window, a 16-point font and a field of view
of about 85° (scroll in slightly). Rebuild the 1280×640 social card with
`bash scripts/social-preview.sh` (ImageMagick, librsvg and fontconfig required).
For a short video, drag the sky, scroll to zoom, press `H` to reveal the full sky,
and search for `Jupiter`. Keep the footage focused on the application.
