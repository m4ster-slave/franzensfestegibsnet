var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
function generateFingerprint() {
    return __awaiter(this, void 0, void 0, function* () {
        const components = {
            screenRes: `${window.screen.width}x${window.screen.height}`,
            screenColorDepth: window.screen.colorDepth,
            userAgent: navigator.userAgent,
            language: navigator.language,
            platform: navigator.platform,
            timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
            cookiesEnabled: navigator.cookieEnabled,
            doNotTrack: navigator.doNotTrack,
            canvas: yield getCanvasFingerprint(),
            webgl: getWebGLFingerprint(),
            fonts: yield detectAvailableFonts(),
        };
        // Create a string from all components using a more compatible approach
        const fingerprintString = Object.keys(components)
            .map((key) => String(components[key]))
            .join("");
        return yield generateHash(fingerprintString);
    });
}
function getCanvasFingerprint() {
    return __awaiter(this, void 0, void 0, function* () {
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
    });
}
function getWebGLFingerprint() {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
    if (!gl) {
        return null;
    }
    const glContext = gl;
    const info = {
        vendor: glContext.getParameter(glContext.VENDOR),
        renderer: glContext.getParameter(glContext.RENDERER),
        webglVersion: glContext.getParameter(glContext.VERSION),
    };
    return JSON.stringify(info);
}
function detectAvailableFonts() {
    return __awaiter(this, void 0, void 0, function* () {
        const baseFonts = ["monospace", "sans-serif", "serif"];
        const testString = "mmmmmmmmmmlli";
        const testSize = "72px";
        const h = document.getElementsByTagName("body")[0];
        const s = document.createElement("span");
        s.style.fontSize = testSize;
        s.innerHTML = testString;
        const metrics = {};
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
    });
}
function generateHash(str) {
    return __awaiter(this, void 0, void 0, function* () {
        const msgBuffer = new TextEncoder().encode(str);
        const hashBuffer = yield crypto.subtle.digest("SHA-256", msgBuffer);
        const hashArray = Array.from(new Uint8Array(hashBuffer));
        // Using a more compatible string padding approach
        return hashArray.map((b) => ("00" + b.toString(16)).slice(-2)).join("");
    });
}
function getFingerprint() {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            return yield generateFingerprint();
        }
        catch (error) {
            console.error("Error generating fingerprint:", error);
            return "error-generating-fingerprint";
        }
    });
}
export { getFingerprint };
