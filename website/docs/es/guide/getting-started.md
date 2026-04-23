# Inicio rápido

## Instalación

Descarga el último instalador desde la [página de descarga](/es/download) o [GitHub Releases](https://github.com/polite0803/AxAgent/releases).

### macOS

| Chip | Archivo |
|------|---------|
| Apple Silicon (M1 / M2 / M3 / M4) | `AxAgent_x.x.x_aarch64.dmg` |
| Intel | `AxAgent_x.x.x_x64.dmg` |

1. Abre el `.dmg` y arrastra **AxAgent** a la carpeta **Aplicaciones**.
2. Inicia AxAgent. Si macOS bloquea la app, ve a **Configuración del Sistema → Privacidad y Seguridad** y haz clic en **Abrir de todas formas**.

::: warning macOS: "La app está dañada" o "No se puede verificar el desarrollador"
Si ves alguno de estos mensajes, abre Terminal y ejecuta:

```bash
xattr -c /Applications/AxAgent.app
```

Luego inicia la app de nuevo. Esto elimina la marca de cuarentena que macOS aplica a las descargas no firmadas.
:::

### Windows

| Arquitectura | Archivo |
|-------------|---------|
| x64 (la mayoría de PCs) | `AxAgent_x.x.x_x64-setup.exe` |
| ARM64 | `AxAgent_x.x.x_arm64-setup.exe` |

Ejecuta el instalador y sigue el asistente. Inicia AxAgent desde el menú Inicio o el acceso directo del escritorio.

### Linux

| Formato | Arquitectura | Archivo |
|---------|-------------|---------|
| Debian / Ubuntu | x64 | `AxAgent_x.x.x_amd64.deb` |
| Debian / Ubuntu | ARM64 | `AxAgent_x.x.x_arm64.deb` |
| Fedora / openSUSE | x64 | `AxAgent_x.x.x_x86_64.rpm` |
| Fedora / openSUSE | ARM64 | `AxAgent_x.x.x_aarch64.rpm` |
| Cualquier distro | x64 | `AxAgent_x.x.x_amd64.AppImage` |
| Cualquier distro | ARM64 | `AxAgent_x.x.x_aarch64.AppImage` |

```bash
# Debian / Ubuntu
sudo dpkg -i AxAgent_x.x.x_amd64.deb

# Fedora / openSUSE
sudo rpm -i AxAgent_x.x.x_x86_64.rpm

# AppImage (cualquier distro)
chmod +x AxAgent_x.x.x_amd64.AppImage
./AxAgent_x.x.x_amd64.AppImage
```

---

## Configuración inicial

### 1. Abrir configuración

Inicia AxAgent y haz clic en el **ícono de engranaje** en la parte inferior de la barra lateral, o presiona <kbd>Cmd/Ctrl</kbd>+<kbd>,</kbd>.

### 2. Añadir un proveedor

Navega a **Configuración → Proveedores** y haz clic en el botón **+**.

1. Introduce un nombre para mostrar (ej. *OpenAI*).
2. Selecciona el tipo de proveedor (OpenAI, Anthropic, Google Gemini, etc.).
3. Pega tu clave API.
4. Confirma la **URL base** — el endpoint oficial está prellenado para los tipos integrados.

::: tip
Puedes añadir tantos proveedores como quieras. Cada proveedor gestiona su propio conjunto de claves API y modelos de forma independiente.
:::

### 3. Obtener modelos

Haz clic en **Obtener modelos** para obtener la lista de modelos disponibles de la API del proveedor. También puedes añadir IDs de modelos manualmente si es necesario.

### 4. Establecer un modelo predeterminado

Ve a **Configuración → Modelo predeterminado** y elige el proveedor y el modelo que las nuevas conversaciones deben usar por defecto.

---

## Tu primera conversación

1. Haz clic en **Nuevo chat** en la barra lateral (o presiona <kbd>Cmd/Ctrl</kbd>+<kbd>N</kbd>).
2. Selecciona un modelo del selector de modelos en la parte superior del chat.
3. Escribe un mensaje y presiona <kbd>Enter</kbd>.
4. AxAgent transmite la respuesta en tiempo real. Los modelos que soportan bloques de pensamiento (ej. Claude, DeepSeek R1) muestran el proceso de razonamiento en una sección plegable.

---

## Atajos

| Atajo | Acción |
|-------|--------|
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> | Mostrar / ocultar ventana actual |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Alt</kbd>+<kbd>A</kbd> | Mostrar / ocultar todas las ventanas |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>W</kbd> | Cerrar ventana |
| <kbd>Cmd/Ctrl</kbd>+<kbd>N</kbd> | Nueva conversación |
| <kbd>Cmd/Ctrl</kbd>+<kbd>,</kbd> | Abrir configuración |
| <kbd>Cmd/Ctrl</kbd>+<kbd>K</kbd> | Paleta de comandos |
| <kbd>Cmd/Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd> | Alternar pasarela API |

---

## Datos y copia de seguridad

### Directorios de datos

| Ruta | Contenido |
|------|----------|
| `~/.axagent/` | Estado de la aplicación — base de datos, claves de cifrado, base vectorial, certificados SSL |
| `~/Documents/axagent/` | Archivos de usuario — imágenes, documentos, copias de seguridad |

---

## Próximos pasos

- [Configurar proveedores](./providers) — añadir y gestionar proveedores de IA
- [Servidores MCP](./mcp) — conectar herramientas externas para ampliar las capacidades de IA
- [Pasarela API](./gateway) — exponer tus proveedores como servidor API local
