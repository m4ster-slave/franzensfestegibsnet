import { getFingerprint } from "/public/js/fingerprinting.js";
import {
  appendImageMarkdown,
  uploadSelectedImage,
} from "/public/js/uploads.js";

const form = document.getElementById("postForm") as HTMLFormElement;
const submitButton = document.getElementById(
  "submitButton",
) as HTMLButtonElement | null;
let fingerprint: string;

async function initForm() {
  fingerprint = await getFingerprint();

  form.addEventListener("submit", async (e) => {
    e.preventDefault();

    try {
      const content = document.getElementById("content") as HTMLTextAreaElement | null;
      const fileInput = form.querySelector(
        "[data-upload-image]",
      ) as HTMLInputElement | null;
      const result = form.querySelector(".upload-result") as HTMLElement | null;

      if (submitButton) submitButton.disabled = true;

      if (fileInput?.files?.length) {
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
      const result = form.querySelector(".upload-result") as HTMLElement | null;
      if (result) {
        result.textContent =
          error instanceof Error ? error.message : "Could not submit report.";
      }
      console.error("Error submitting form:", error);
    } finally {
      if (submitButton) submitButton.disabled = false;
    }
  });
}

if (form) {
  initForm();
}
