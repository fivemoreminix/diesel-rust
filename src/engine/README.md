# Diesel Engine
The Diesel Engine is a sub-module of the Diesel text editor. The engine handles low-level interactions, and exposes them as high-level abstractions. This makes it easy to prototype and refactor in the editor. Many tests are written in the engine to harden the code.

## Abstractions
 * Abstraction over the low-level terminal manipulation library in use. Any backend could be dropped in place, and all other code will work properly. The default backend is Crossterm.
 * Primitives rendering (drawing shapes and other high-level objects)
 * Reading and writing to files.
