import { getFingerprint } from "/public/js/fingerprinting.js";

const form = document.getElementById("postForm") as HTMLFormElement;
let fingerprint: string;

async function initForm() {
  fingerprint = await getFingerprint();

  form.addEventListener("submit", async (e) => {
    e.preventDefault();

    try {
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
      console.error("Error submitting form:", error);
    }
  });
}

initForm();
