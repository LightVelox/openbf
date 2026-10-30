Menu cursor ("Ion / Clear", original art). `menu_cursor.svg` is the only file: the game renders it at
startup at the display's size (32 px at 100 %, 48 at 150 %, 64 at 200 %, ...) and hands that to the OS
as the cursor. Hotspot (3, 3) in the SVG's 32-unit space. To replace the cursor, replace the SVG and
keep a 32 x 32 viewBox with the tip at (3, 3) (or change `HOTSPOT` in `src/main/windowed/mouse.rs`).
