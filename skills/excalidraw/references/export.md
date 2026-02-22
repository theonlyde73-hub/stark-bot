# Export to PNG/SVG

Export `.excalidraw` files to PNG and/or SVG using a Node.js script with `@excalidraw/utils`.

## Method 1: Node.js Script (Recommended)

Use the `exec` tool to run a Node.js export script.

### Export to SVG

```tool:exec
command: |
  node -e "
  const fs = require('fs');
  const data = JSON.parse(fs.readFileSync('EXCALIDRAW_FILE_PATH', 'utf8'));
  import('https://esm.sh/@excalidraw/utils@0.1.2').then(async (utils) => {
    const { exportToSvg } = utils.default;
    const svg = await exportToSvg({
      elements: data.elements,
      appState: { ...data.appState, exportBackground: true },
      files: data.files || {}
    });
    fs.writeFileSync('OUTPUT_PATH.svg', svg.outerHTML);
    console.log('SVG exported successfully');
  });
  "
```

### Export to PNG via Base64

```tool:exec
command: |
  node -e "
  const fs = require('fs');
  const data = JSON.parse(fs.readFileSync('EXCALIDRAW_FILE_PATH', 'utf8'));
  import('https://esm.sh/@excalidraw/utils@0.1.2').then(async (utils) => {
    const { exportToBlob } = utils.default;
    const blob = await exportToBlob({
      elements: data.elements,
      appState: { ...data.appState, exportBackground: true },
      files: data.files || {},
      mimeType: 'image/png'
    });
    const buffer = Buffer.from(await blob.arrayBuffer());
    fs.writeFileSync('OUTPUT_PATH.png', buffer);
    console.log('PNG exported successfully');
  });
  "
```

## Method 2: Playwright MCP (If Available)

If Playwright MCP tools are available (`browser_navigate`, `browser_run_code`, `browser_close`):

### 1. Start a Local HTTP Server

```bash
python3 -m http.server 8765 &
SERVER_PID=$!
```

### 2. Navigate Playwright to the Server

```
browser_navigate -> http://localhost:8765/
```

The 404 page is fine - we only need the HTTP origin for dynamic import to work.

### 3. Read the .excalidraw File

Use `read_file` to get the `.excalidraw` file contents as a string.

### 4. Export SVG

Use `browser_run_code`:

```javascript
async (page) => {
  const excalidrawJson = `EXCALIDRAW_JSON_HERE`;

  const svgString = await page.evaluate(async (json) => {
    const utils = await import('https://esm.sh/@excalidraw/utils@0.1.2');
    const { exportToSvg } = utils.default;
    const data = JSON.parse(json);
    const svg = await exportToSvg({
      elements: data.elements,
      appState: { ...data.appState, exportBackground: true },
      files: data.files || {}
    });
    return svg.outerHTML;
  }, excalidrawJson);

  return svgString;
}
```

Write the returned SVG string to `<filename>.svg`.

### 5. Export PNG

Use `browser_run_code`:

```javascript
async (page) => {
  const excalidrawJson = `EXCALIDRAW_JSON_HERE`;

  const pngBase64 = await page.evaluate(async (json) => {
    const utils = await import('https://esm.sh/@excalidraw/utils@0.1.2');
    const { exportToBlob } = utils.default;
    const data = JSON.parse(json);
    const blob = await exportToBlob({
      elements: data.elements,
      appState: { ...data.appState, exportBackground: true },
      files: data.files || {},
      mimeType: 'image/png'
    });
    const reader = new FileReader();
    return new Promise((resolve) => {
      reader.onloadend = () => resolve(reader.result);
      reader.readAsDataURL(blob);
    });
  }, excalidrawJson);

  return pngBase64;
}
```

Decode the base64 result (strip `data:image/png;base64,` prefix):

```bash
echo "<base64_data_without_prefix>" | base64 -d > <filename>.png
```

### 6. Clean Up

```
browser_close
kill $SERVER_PID
```

## Key Details

- **Import path**: Export functions are on `utils.default`, not named exports
- **Console errors**: `<text> attribute y: Expected length` warnings are cosmetic
- **Background**: `exportBackground: true` includes the white background
- **Output location**: Save alongside the `.excalidraw` file with matching name
- **Visual fidelity**: Both exports match excalidraw.com rendering

## Troubleshooting

| Issue | Fix |
|-------|-----|
| Port already in use | Try a different port: `python3 -m http.server 9876 &` |
| Dynamic import fails | Check network connectivity; `esm.sh` CDN must be reachable |
| PNG is blank/corrupted | Verify base64 prefix was stripped before decoding |
| SVG missing text | Cosmetic only - text renders correctly in browser |
