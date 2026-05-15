const uploadForm = document.querySelector("[data-archive-upload]");
const fileInput = document.querySelector("[data-archive-files]");
const dropzone = document.querySelector("[data-dropzone]");
const fileSummary = document.querySelector("[data-archive-file-summary]");
const archiveManager = document.querySelector("[data-archive-manager]");
const filterInput = document.querySelector("[data-archive-filter]");
const previewToggle = document.querySelector("[data-archive-preview-toggle]");
const layoutButtons = document.querySelectorAll("[data-archive-layout]");

const layoutKey = "archiveLayout";
const previewKey = "archiveShowPreviews";

function updateFileSummary() {
  if (!fileInput || !fileSummary) return;

  const count = fileInput.files ? fileInput.files.length : 0;
  fileSummary.textContent =
    count === 0 ? "No files selected." : `${count} file${count === 1 ? "" : "s"} ready.`;
}

if (fileInput) {
  fileInput.addEventListener("change", updateFileSummary);
}

if (dropzone && fileInput) {
  ["dragenter", "dragover"].forEach((eventName) => {
    dropzone.addEventListener(eventName, (event) => {
      event.preventDefault();
      dropzone.classList.add("archive-dropzone--active");
    });
  });

  ["dragleave", "drop"].forEach((eventName) => {
    dropzone.addEventListener(eventName, (event) => {
      event.preventDefault();
      dropzone.classList.remove("archive-dropzone--active");
    });
  });

  dropzone.addEventListener("drop", (event) => {
    if (!event.dataTransfer?.files?.length) return;
    fileInput.files = event.dataTransfer.files;
    updateFileSummary();
  });
}

if (uploadForm) {
  uploadForm.addEventListener("submit", (event) => {
    if (!fileInput?.files?.length) {
      event.preventDefault();
      if (fileSummary) fileSummary.textContent = "Choose at least one file first.";
    }
  });
}

function applyArchiveView() {
  if (!archiveManager) return;

  const layout = localStorage.getItem(layoutKey) || "list";
  const showPreviews = localStorage.getItem(previewKey) === "true";

  archiveManager.classList.toggle("archive-view-list", layout === "list");
  archiveManager.classList.toggle("archive-view-grid", layout === "grid");
  archiveManager.classList.toggle("archive-preview-off", !showPreviews);

  if (previewToggle) previewToggle.checked = showPreviews;

  layoutButtons.forEach((button) => {
    const active = button.getAttribute("data-archive-layout") === layout;
    button.classList.toggle("archive-control-active", active);
    button.setAttribute("aria-pressed", active ? "true" : "false");
  });
}

function filterArchiveFiles() {
  const query = (filterInput?.value || "").trim().toLowerCase();

  document.querySelectorAll("[data-archive-file]").forEach((file) => {
    const text = (file.getAttribute("data-search-text") || "").toLowerCase();
    file.hidden = query.length > 0 && !text.includes(query);
  });
}

layoutButtons.forEach((button) => {
  button.addEventListener("click", () => {
    localStorage.setItem(layoutKey, button.getAttribute("data-archive-layout") || "list");
    applyArchiveView();
  });
});

if (previewToggle) {
  previewToggle.addEventListener("change", () => {
    localStorage.setItem(previewKey, previewToggle.checked ? "true" : "false");
    applyArchiveView();
  });
}

if (filterInput) {
  filterInput.addEventListener("input", filterArchiveFiles);
}

document.querySelectorAll("[data-copy-value]").forEach((button) => {
  button.addEventListener("click", async () => {
    const value = button.getAttribute("data-copy-value") || "";

    try {
      await navigator.clipboard.writeText(value);
      button.textContent = "Copied";
      window.setTimeout(() => {
        button.textContent = "Copy";
      }, 1400);
    } catch (_) {
      const input = button.parentElement?.querySelector("input");
      if (input) {
        input.focus();
        input.select();
      }
    }
  });
});

applyArchiveView();
filterArchiveFiles();

document.querySelectorAll("[data-delete-file]").forEach((form) => {
  form.addEventListener("submit", (event) => {
    const filename = form.getAttribute("data-delete-file") || "this file";
    const confirmed = window.confirm(
      `Delete "${filename}" permanently? Existing links will stop working.`,
    );

    if (!confirmed) {
      event.preventDefault();
    }
  });
});
