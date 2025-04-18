function setRealVh() {
  const height = window.visualViewport ? window.visualViewport.height : window.innerHeight;
  document.documentElement.style.setProperty('--vh', `${height * 0.01}px`);
}

window.addEventListener('load', setRealVh);
window.visualViewport?.addEventListener('resize', setRealVh);
window.visualViewport?.addEventListener('scroll', setRealVh); 
document.addEventListener('focusin', setRealVh);
document.addEventListener('focusout', setRealVh);