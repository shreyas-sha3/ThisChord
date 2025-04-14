const gruvboxTheme = {
    "--bg": "#282828",
    "--fg": "#ebdbb2",
    "--dark-gray": "#3c3836",
    "--light-gray": "#a89984",
    "--yellow": "#d79921",
    "--green": "#b8bb26",
    "--blue": "#83a598",
    "--red": "#fb4934",
    "--orange": "#fe8019",
};

const rosePineTheme = {
    "--bg": "#191724",          // deeper than default, matches gruv dark
    "--fg": "#e0def4",          // readable soft white
    "--dark-gray": "#1f1d2e",   // muted contrast layer
    "--light-gray": "#9a89c2",  // soft lavender highlight
    "--yellow": "#f6c177",      // golden peach, kept
    "--green": "#3e8fb0",       // deeper teal for action buttons
    "--blue": "#569fba",        // brighter and punchier than 31748f
    "--red": "#eb6f92",         // matches well as-is
    "--orange": "#f2975c",      // warmer, closer to gruv orange
};

const nordicTheme = {
    "--bg": "#2e3440",
    "--fg": "#d8dee9",
    "--dark-gray": "#3b4252",
    "--light-gray": "#81a1c1", // adjusted to match gruv-ish contrast
    "--yellow": "#ebcb8b",
    "--green": "#a3be8c",
    "--blue": "#5e81ac",       // deeper blue for better vibe
    "--red": "#bf616a",
    "--orange": "#d08770",
};


const themes = [gruvboxTheme, rosePineTheme, nordicTheme];
const themeNames = ["gruvbox", "rosepine", "nordic",];

function applyTheme(theme) {
    const root = document.documentElement;
    for (const [key, value] of Object.entries(theme)) {
        root.style.setProperty(key, value);
    }
}

function loadTheme() {
    const saved = localStorage.getItem("themeIndex");
    return saved ? parseInt(saved) : 0;
}

function saveTheme(index) {
    localStorage.setItem("themeIndex", index);
}

document.addEventListener("DOMContentLoaded", () => {
    let currentThemeIndex = loadTheme();
    applyTheme(themes[currentThemeIndex]);

    const toggleBtn = document.getElementById("themeToggle");
    if (toggleBtn) {
        toggleBtn.addEventListener("click", () => {
            currentThemeIndex = (currentThemeIndex + 1) % themes.length;
            applyTheme(themes[currentThemeIndex]);
            saveTheme(currentThemeIndex);
            console.log(`Switched to: ${themeNames[currentThemeIndex]}`);
        });
    }
});
