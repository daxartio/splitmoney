// Prevent double-tap zoom on mobile to keep press-and-hold gestures snappy.
let lastTouchEnd = 0;
document.addEventListener(
    "touchend",
    (event) => {
        const now = Date.now();
        if (now - lastTouchEnd <= 350) {
            event.preventDefault();
        }
        lastTouchEnd = now;
    },
    { passive: false },
);

// Guard against pinch-zoom gesture.
document.addEventListener(
    "gesturestart",
    (event) => {
        event.preventDefault();
    },
    { passive: false },
);
