# Guía de Contribución y Feedback - Kore Package Manager (kpm)

¡Gracias por querer ayudar a mejorar **Kore Package Manager**! Este proyecto es de código abierto y toda ayuda es bienvenida, ya sea reportando errores, sugiriendo mejoras o escribiendo código.

## Cómo ayudar con el programa

Hay varias formas en las que puedes contribuir al desarrollo de `kpm`:

1.  **Reportar Errores (Bugs):** Si algo no funciona como debería, háznoslo saber.
2.  **Sugerir Funcionalidades:** ¿Tienes una idea genial? ¡Compártela!
3.  **Mejorar la Documentación:** Corregir errores tipográficos o añadir ejemplos útiles.
4.  **Enviar Pull Requests:** Si eres desarrollador, puedes clonar el repositorio, hacer tus cambios y enviarlos para revisión.

---

## Cómo dejar Feedback correctamente

El feedback es vital para que el programa crezca de forma saludable. Para que tu feedback sea útil y podamos procesarlo rápido, sigue estas pautas:

### 1. ¿Dónde dejar el feedback?
La mejor forma es a través de las **GitHub Issues** del repositorio oficial. 
*   **Issues de Error (Bug Reports):** Para reportar fallos técnicos.
*   **Issues de Sugerencia (Feature Requests):** Para nuevas ideas o cambios de diseño.

### 2. ¿Qué incluir en un buen reporte?
Para que podamos ayudarte (o implementar tu idea), necesitamos contexto. Por favor, incluye lo siguiente:

*   **Versión de kpm:** Ejecuta `kpm -V` y copia la salida.
*   **Sistema Operativo:** Tu distribución de Linux (ej: Arch Linux, Void, Fedora) y entorno de escritorio (GNOME, KDE, etc.).
*   **Descripción Clara:** Explica qué estabas haciendo cuando ocurrió el problema o qué quieres lograr con tu sugerencia.
*   **Pasos para reproducir (en caso de errores):**
    1. Ejecuté `kpm install ...`
    2. Seleccioné la opción X...
    3. El programa se cerró con el error Y.
*   **Comportamiento esperado vs. real:** "¿Qué debería haber pasado?" vs "¿Qué pasó realmente?".
*   **Capturas de pantalla o logs:** Si el error ocurre en la TUI, una captura de pantalla ayuda muchísimo.

### 3. Mantén la cordialidad
Recuerda que este es un proyecto mantenido por la comunidad. Seamos amables y constructivos en todas nuestras interacciones.

---

## ¿Quieres ayudar a aumentar los paquetes de kpm? ¡Haz esto!

Si conoces una aplicación que se distribuye en tarball y no está en nuestra lista, puedes añadirla fácilmente:

1.  **Haz un Fork** de este repositorio.
2.  **Edita el archivo**: Abre [`assets/community_repos.json`](file:///home/ezequiel/Documentos/Kore-Package-Manager/assets/community_repos.json).
3.  **Añade la aplicación**: Agrega un nuevo objeto JSON al final de la lista `repositories` siguiendo este formato:
    ```json
    {
      "name": "Nombre Visual",
      "package_name": "nombre-paquete",
      "url": "https://github.com/ezequielgk/Kore-Package-Manager",
      "category": "Utility",
      "requires_root": false,
      "terminal": false,
      "description": "Una breve descripción de lo que hace la app."
    }
    ```
    > **Nota sobre la URL**: Preferimos enlaces directos a repositorios (ej: GitHub). No pongas URLs a páginas externas; en caso de que sea necesario usar una, por favor explícalo en el chat de tu Pull Request.

4.  **Crea un Pull Request**: Envía tus cambios con el título "Add package: [nombre]".

---

¡Gracias por ser parte de Kore Package Manager! 🦀
