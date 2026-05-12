function insertAtCursor(textarea: HTMLTextAreaElement, value: string) {
  const start = textarea.selectionStart || textarea.value.length;
  const end = textarea.selectionEnd || textarea.value.length;
  textarea.value = `${textarea.value.slice(0, start)}${value}${textarea.value.slice(end)}`;
  textarea.focus();
  textarea.selectionStart = start + value.length;
  textarea.selectionEnd = start + value.length;
}

function uploadErrorMessage(response: Response): string {
  if (response.status === 413) {
    return "Image is too large. Try one under 12 MB.";
  }

  if (response.status === 415) {
    return "Use PNG, JPG, WebP, or GIF.";
  }

  return "Image upload failed.";
}

export async function uploadSelectedImage(
  fileInput: HTMLInputElement | null,
  result: HTMLElement | null,
): Promise<string | null> {
  if (!fileInput?.files?.length) {
    return null;
  }

  const formData = new FormData();
  formData.append("file", fileInput.files[0]);

  if (result) result.textContent = "Uploading image...";

  const response = await fetch("/uploads/images", {
    method: "POST",
    body: formData,
  });

  if (!response.ok) {
    throw new Error(uploadErrorMessage(response));
  }

  const data = await response.json();
  if (result) result.textContent = "Image attached.";
  return data.path;
}

export function appendImageMarkdown(
  textarea: HTMLTextAreaElement | null,
  path: string | null,
) {
  if (!textarea || !path) return;
  insertAtCursor(textarea, `\n![uploaded image](${path})\n`);
}

document.querySelectorAll("[data-upload-on-submit]").forEach((form) => {
  form.addEventListener("submit", async (event) => {
    const htmlForm = form as HTMLFormElement;

    if (htmlForm.dataset.uploadComplete === "true") {
      return;
    }

    const targetId = htmlForm.getAttribute("data-upload-target");
    const target = targetId
      ? (document.getElementById(targetId) as HTMLTextAreaElement | null)
      : null;
    const result = htmlForm.querySelector(".upload-result") as HTMLElement | null;
    const fileInput = htmlForm.querySelector(
      "[data-upload-image]",
    ) as HTMLInputElement | null;

    if (!fileInput?.files?.length) {
      return;
    }

    event.preventDefault();

    try {
      const path = await uploadSelectedImage(fileInput, result);
      appendImageMarkdown(target, path);
      fileInput.value = "";
      htmlForm.dataset.uploadComplete = "true";
      htmlForm.requestSubmit();
    } catch (error) {
      if (result) {
        result.textContent =
          error instanceof Error ? error.message : "Image upload failed.";
      }
    }
  });
});
