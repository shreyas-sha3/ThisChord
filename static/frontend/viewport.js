//SET VIEWPORT HEIGHT
function setVh() {
    const vh = window.innerHeight * 0.01;
    document.documentElement.style.setProperty('--vh', `${vh}px`);
  }

  window.addEventListener('load', setVh);
  window.addEventListener('resize', setVh);
  window.addEventListener('orientationchange', setVh);

