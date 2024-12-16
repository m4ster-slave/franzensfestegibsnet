import { getFingerprint } from "/public/js/fingerprinting.js";

const form = document.getElementById("postForm") as HTMLFormElement;
const inputElement = document.getElementById("fingerprint") as HTMLInputElement;

async function setFingerprint() {
  const fingerprint = await getFingerprint();
  inputElement.value = fingerprint;
}

setFingerprint();
