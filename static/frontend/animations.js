// animations.js

// Utility to fade in an element
export function fadeIn(el, duration = 500) {
    el.style.opacity = 0;
    el.style.transition = `opacity ${duration}ms`;
    requestAnimationFrame(() => (el.style.opacity = 1));
}

// Slide in from bottom (like messages)
export function slideInUp(el, duration = 350) {
    el.style.transform = "translateY(20px)";
    el.style.opacity = 0;
    el.style.transition = `all ${duration}ms ease-out`;
    requestAnimationFrame(() => {
        el.style.transform = "translateY(0)";
        el.style.opacity = 1;
    });
}

export function shake(el) {
    el.classList.add("shake");
    setTimeout(() => el.classList.remove("shake"), 150);
}

