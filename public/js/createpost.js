import { getFingerprint } from "/js/fingerprinting.js";
import { appendImageMarkdown, uploadSelectedImage } from "/js/uploads.js";

const form = document.getElementById("postForm");
const submitButton = document.getElementById("submitButton");
let fingerprint;

async function initForm() {
  fingerprint = await getFingerprint();

  form.addEventListener("submit", async (event) => {
    event.preventDefault();

    const content = document.getElementById("content");
    const fileInput = form.querySelector("[data-upload-image]");
    const result = form.querySelector(".upload-result");

    try {
      if (submitButton) submitButton.disabled = true;

      if (fileInput && fileInput.files && fileInput.files.length) {
        const path = await uploadSelectedImage(fileInput, result);
        appendImageMarkdown(content, path);
        fileInput.value = "";
      }

      const formData = new FormData(form);
      const response = await fetch("/forum/create", {
        method: "POST",
        headers: {
          "X-Fingerprint": fingerprint,
        },
        body: formData,
      });

      if (response.redirected) {
        window.location.href = response.url;
      } else {
        const data = await response.text();
        document.body.innerHTML = data;
      }
    } catch (error) {
      if (result) result.textContent = error.message || "Could not submit report.";
      console.error("Error submitting form:", error);
    } finally {
      if (submitButton) submitButton.disabled = false;
    }
  });
}

if (form) {
  initForm();
}
