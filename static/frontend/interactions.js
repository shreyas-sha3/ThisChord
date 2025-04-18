// interactions.js
export function setupButtonPressEffect(selector = '.server-btn') {
    document.addEventListener('click', (e) => {
      const btn = e.target.closest(selector); 
      if (btn) {
        btn.style.transform = 'translateY(2px)';
        setTimeout(() => {
          btn.style.transform = '';
        }, 100);
      }
    });
  }
  //Send Button Ripple Effect
const sendBtn = document.getElementById("sendBtn");

sendBtn.addEventListener("click", function (e) {
  const circle = document.createElement("span");
  const rect = this.getBoundingClientRect();
  const size = Math.max(rect.width, rect.height);

  circle.classList.add("ripple");
  circle.style.width = circle.style.height = `${size}px`;
  circle.style.left = `${e.clientX - rect.left - size / 2}px`;
  circle.style.top = `${e.clientY - rect.top - size / 2}px`;

  this.appendChild(circle);

  setTimeout(() => circle.remove(), 600);
});
