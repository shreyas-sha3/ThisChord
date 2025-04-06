let url = "://rust-chat-um86.onrender.com";

const form = document.getElementById("auth-form");
const msg = document.getElementById("log-message");
const toggleBtn = document.getElementById("log-reg-toggle");
const formTitle = document.getElementById("form-title");

let mode = "login"; // 'login' or 'register'

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  const formData = new FormData(form);
  const data = new URLSearchParams(formData);

  const response = await fetch(`http${url}/${mode}`, {
    method: "POST",
    body: data,
    credentials: "include",
  });

  const text = await response.text();

  if (mode === "login" && text.includes("Login successful")) {
    window.location.href = "/chat.html";
  } else {
    msg.textContent = text;
  }
});

toggleBtn.addEventListener("click", () => {
  mode = mode === "login" ? "register" : "login";
  formTitle.textContent = mode.charAt(0).toUpperCase() + mode.slice(1);
  form.querySelector("button[type='submit']").textContent = mode.charAt(0).toUpperCase() + mode.slice(1);
  toggleBtn.textContent = mode === "login" ? "Switch to Register" : "Switch to Login";
  msg.textContent = "";
});
