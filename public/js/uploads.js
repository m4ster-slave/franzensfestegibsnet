function insertAtCursor(textarea, value) {
  const start = textarea.selectionStart || textarea.value.length;
  const end = textarea.selectionEnd || textarea.value.length;
  textarea.value = `${textarea.value.slice(0, start)}${value}${textarea.value.slice(end)}`;
  textarea.focus();
  textarea.selectionStart = start + value.length;
  textarea.selectionEnd = start + value.length;
}

document.querySelectorAll("[data-upload-target]").forEach((form) => {
  form.addEventListener("submit", async (event) => {
    event.preventDefault();

    const targetId = form.getAttribute("data-upload-target");
    const target = document.getElementById(targetId);
    const result = form.querySelector(".upload-result");
    const fileInput = form.querySelector('input[type="file"]');

    if (!target || !fileInput || !fileInput.files.length) {
      if (result) result.textContent = "Choose an image first.";
      return;
    }

    const formData = new FormData();
    formData.append("file", fileInput.files[0]);

    if (result) result.textContent = "Uploading...";

    const response = await fetch("/uploads/images", {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      if (result) result.textContent = "Upload failed.";
      return;
    }

    const data = await response.json();
    insertAtCursor(target, `\n![uploaded image](${data.path})\n`);
    fileInput.value = "";
    if (result) result.textContent = "Inserted.";
  });
});
