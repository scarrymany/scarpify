<p align="center">
  <img src="assets/icon.svg" width="96" alt="Логотип SCARPIFY">
</p>

<h1 align="center">SCARPIFY</h1>

<p align="center">
  Музыкальный плеер с открытым исходным кодом для YouTube Music и SoundCloud.<br>
  Без рекламы, без аккаунтов, одна медиатека.
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="https://github.com/scarrymany/scarpify/releases/latest">Скачать</a>
</p>

## Возможности

- Поиск сразу по YouTube Music и SoundCloud. У каждого трека указан сервис, откуда он взят.
- Импорт публичных плейлистов и альбомов Spotify по ссылке, без аккаунта Spotify. Треки подбираются и проигрываются через YouTube Music.
- Любимые треки, недавно прослушанное, очередь, перемешивание и повтор.
- Выравнивание громкости по данным YouTube.
- Тёмная, светлая и системная тема, шесть акцентных цветов, анимации можно выключить.
- Интерфейс на русском, украинском и английском.
- Минимальная нагрузка: звук декодируется нативно на Rust, интерфейс работает в одном лёгком WebView.

## Установка

Скачайте последний `SCARPIFY_x.y.z_x64-setup.exe` со [страницы релизов](https://github.com/scarrymany/scarpify/releases/latest) и запустите. Права администратора не нужны.

Поддерживаются Windows 10 и 11 (x64). Если WebView2 не установлен, установщик поставит его сам.

## Горячие клавиши

| Действие | Клавиши |
| --- | --- |
| Пауза / продолжить | `Пробел` |
| Следующий / предыдущий трек | `Ctrl + →` / `Ctrl + ←` |
| Поиск | `Ctrl + L`, `Ctrl + F` или `/` |
| Назад / вперёд | `Alt + ←` / `Alt + →`, боковые кнопки мыши |

## Сборка из исходников

Нужны [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 20+ и [зависимости Tauri](https://tauri.app/start/prerequisites/) для вашей системы.

```bash
npm install
npm run tauri dev      # сборка для разработки с горячей перезагрузкой
npm run tauri build    # установщик в src-tauri/target/release/bundle
```

## Отказ от ответственности

SCARPIFY не связан с YouTube, Google, SoundCloud или Spotify. Плеер обращается к общедоступным интерфейсам так же, как их веб-клиенты. Используйте его в соответствии с условиями сервисов и законами вашей страны.

## Лицензия

[GPL-3.0-or-later](LICENSE).
