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
  