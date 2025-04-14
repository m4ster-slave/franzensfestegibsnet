var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { getFingerprint } from "/public/js/fingerprinting.js";
const form = document.getElementById("postForm");
let fingerprint;
function initForm() {
    return __awaiter(this, void 0, void 0, function* () {
        fingerprint = yield getFingerprint();
        form.addEventListener("submit", (e) => __awaiter(this, void 0, void 0, function* () {
            e.preventDefault();
            try {
                const formData = new FormData(form);
                const response = yield fetch("/forum/create", {
                    method: "POST",
                    headers: {
                        "X-Fingerprint": fingerprint,
                    },
                    body: formData,
                });
                if (response.redirected) {
                    window.location.href = response.url;
                }
                else {
                    const data = yield response.text();
                    document.body.innerHTML = data;
                }
            }
            catch (error) {
                console.error("Error submitting form:", error);
            }
        }));
    });
}
initForm();
