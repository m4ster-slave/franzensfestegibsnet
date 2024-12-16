interface FingerprintComponents {
  screenRes: string;
  screenColorDepth: number;
  userAgent: string;
  language: string;
  platform: string;
  timezone: string;
  cookiesEnabled: boolean;
  doNotTrack: string | null;
  canvas: string;
  webgl: string | null;
  fonts: string;
}

interface WebGLInfo {
  vendor: string | null;
  renderer: string | null;
  webglVersion: string | null;
}

interface FontMetrics {
  [key: string]: {
    width: number;
    height: number;
  };
}

async function generateFingerprint(): Promise<string> {
  const components: FingerprintComponents = {
    screenRes: `${window.screen.width}x${window.screen.height}`,
    screenColorDepth: window.screen.colorDepth,
    userAgent: navigator.userAgent,
    language: navigator.language,
    platform: navigator.platform,
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    cookiesEnabled: navigator.cookieEnabled,
    doNotTrack: navigator.doNotTrack,
    canvas: await getCanvasFingerprint(),
    webgl: getWebGLFingerprint(),
    fonts: await detectAvailableFonts(),
  };

  // Create a string from all components using a more compatible approach
  const fingerprintString = Object.keys(components)
    .map((key) => String((components as any)[key]))
    .join("");

  return await generateHash(fingerprintString);
}

async function getCanvasFingerprint(): Promise<string> {
  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d");

  if (!ctx) {
    return "canvas-not-supported";
  }

  ctx.textBaseline = "top";
  ctx.font = "14px Arial";
  ctx.fillStyle = "#f60";
  ctx.fillRect(125, 1, 62, 20);
  ctx.fillStyle = "#069";
  ctx.fillText("Hello, world!", 2, 15);
  ctx.fillStyle = "rgba(102, 204, 0, 0.7)";
  ctx.fillText("Hello, world!", 4, 17);

  return canvas.toDataURL();
}

function getWebGLFingerprint(): string | null {
  const canvas = document.createElement("canvas");
  const gl =
    canvas.getContext("webgl") || canvas.getContext("experimental-webgl");

  if (!gl) {
    return null;
  }

  const glContext = gl as WebGLRenderingContext;

  const info: WebGLInfo = {
    vendor: glContext.getParameter(glContext.VENDOR),
    renderer: glContext.getParameter(glContext.RENDERER),
    webglVersion: glContext.getParameter(glContext.VERSION),
  };

  return JSON.stringify(info);
}

async function detectAvailableFonts(): Promise<string> {
  const baseFonts = ["monospace", "sans-serif", "serif"];
  const testString = "mmmmmmmmmmlli";
  const testSize = "72px";
  const h = document.getElementsByTagName("body")[0];

  const s = document.createElement("span");
  s.style.fontSize = testSize;
  s.innerHTML = testString;

  const metrics: FontMetrics = {};

  for (const baseFont of baseFonts) {
    s.style.fontFamily = baseFont;
    h.appendChild(s);
    metrics[baseFont] = {
      width: s.offsetWidth,
      height: s.offsetHeight,
    };
    h.removeChild(s);
  }

  return JSON.stringify(metrics);
}

async function generateHash(str: string): Promise<string> {
  const msgBuffer = new TextEncoder().encode(str);
  const hashBuffer = await crypto.subtle.digest("SHA-256", msgBuffer);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  // Using a more compatible string padding approach
  return hashArray.map((b) => ("00" + b.toString(16)).slice(-2)).join("");
}

async function getFingerprint(): Promise<string> {
  try {
    return await generateFingerprint();
  } catch (error) {
    console.error("Error generating fingerprint:", error);
    return "error-generating-fingerprint";
  }
}

export { getFingerprint };
