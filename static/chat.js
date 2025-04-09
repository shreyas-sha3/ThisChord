//let http_url="http://localhost:8080"
//let ws_url="ws://localhost:8080"
let http_url="https://rust-chat-um86.onrender.com"
let ws_url="wss://rust-chat-um86.onrender.com"

let socket
let username
const statusElement = document.getElementById("username");
const notificationSound = new Audio("./notify.mp3");

//PRE-CHAT
fetch(`${http_url}/auth-check`, {
    credentials: "include"
  })
  .then(res => res.json())
  .then(data => {
    if (data.status === "ok") {
    ConnectSocket()
    username = data.username;
    let usernameElement = document.getElementById("username");
    usernameElement.textContent = username;
    } else {
    window.location.href = "./auth.html";
    }
  });


//CHAT

function ToggleStatus(status){
    if(status==1){
    statusElement.textContent = username;
    //statusElement.style.color="#b8bb26";
    }
    else if(status==0){
        statusElement.textContent = "Connecting...";
        //statusElement.style.color="#83a598";
    }
    else if(status==-1){
        statusElement.textContent = "Disconnected...";
        //statusElement.style.color="#fb4934";
    }
}

let pingInterval;
function ConnectSocket() {
    ToggleStatus(0);
    socket = new WebSocket(`${ws_url}/chat`);
    socket.onopen = () => {
        ToggleStatus(1);
        restoreDMs();

        // Start keep-alive ping every 5 minutes
        pingInterval = setInterval(() => {
            if (socket.readyState === WebSocket.OPEN) {
                socket.send(JSON.stringify({ type: "ping" }));
            }
        }, 10 * 60 * 1000); 
    };

    socket.onclose = () => {
        ToggleStatus(-1);
        clearInterval(pingInterval); 
    };

    socket.onmessage = (event) => {
        //console.log("Message received:", event.data);
        const parsed = JSON.parse(event.data);
        const [sender, messageText, msgType, targetUser] = parsed;
        const key = msgType === "dm"
            ? (sender === username ? targetUser : sender)
            : null;
    
            const messages = loadChat(key);
            messages.push(parsed);
            saveChat(key, messages);

    displayMessage(event.data);

    if (document.hidden) {
        notificationSound.play().catch((err) => {
                console.warn("Notification sound failed to play:", err);
        });
    };
}
}
function sendMessage(event) {
    event.preventDefault();
    const MessageBox = document.getElementById("ClientMessage");
    const msg = MessageBox.value.trim();
    if (msg) {
        const payload = {
            type: msg_type,
            msg: msg
        };
        if (msg_type === "dm") {
            payload.to = dm_recipient;
        }
        socket.send(JSON.stringify(payload));
        MessageBox.value = "";
    }
}


let msg_type = "broadcast", dm_recipient;
dm_recipient = null;

function saveChat(username, messages) {
    const trimmed = messages.slice(-100); // keep only last 100 messages
    localStorage.setItem(`chat_${username || 'public'}`, JSON.stringify(trimmed));
}

function loadChat(username) {
    return JSON.parse(localStorage.getItem(`chat_${username || 'public'}`)) || [];
}

function renderChatMessages(messages) {
    const chatBox = document.getElementById('ChatBox');
    chatBox.innerHTML = '';
    lastUser = "";

    if (messages.length === 0) {
        const empty = document.createElement("p");
        empty.textContent = "No messages yet!";
        empty.classList.add("EmptyMessage");
        chatBox.appendChild(empty);
        return;
    }

    messages.forEach(msg => {
        if (typeof msg === "string") {
            displayMessage(msg);
        } else {
            displayMessage(JSON.stringify(msg));
        }
    });
}


function appendMessage(username, message) {
    const messages = loadChat(username);
    messages.push(message);
    saveChat(username, messages);
    renderChatMessages(messages);
}

function switchToDM(username) {
    const chatTitle = document.getElementById('ChatTitle');
    const messageInput = document.getElementById('ClientMessage');

    if (dm_recipient !== username) {
        msg_type = username ? "dm" : "broadcast";
        dm_recipient = username;
        chatTitle.textContent = username ? `DM: ${dm_recipient}` : 'Public Chat';
        renderChatMessages(loadChat(dm_recipient));
        messageInput.placeholder = username ? `Message ${dm_recipient}...` : `Type a message...`;
    }
    requestAnimationFrame(() => {
        const messagesDiv = document.getElementById("ChatBox");
        messagesDiv.scrollTop = messagesDiv.scrollHeight;
    });
    // Remove notification dot if present
    const dmButtons = document.querySelectorAll(".dm-btn");
    dmButtons.forEach(btn => {
        if (btn.textContent.trim().startsWith(username)) {
            const dot = btn.querySelector(".notification-dot");
            if (dot) dot.remove();
        }
    });
    messageInput.focus();
}


// === Clicking on a chat user ===
document.getElementById("ChatBox").addEventListener("click", (event) => {
    if (event.target.tagName === "BUTTON") {
        let button_text = event.target.textContent.trim();        
        if (button_text.includes("::")) {
            button_text = button_text.split("::").pop().trim();
        }
        createDM(button_text)
        switchToDM(button_text);
    }
});


// Add user to DM list if not already present
function createDM(dmUsername) {
    const dmPane = document.getElementById("dm-pane");
    const existingDM = Array.from(dmPane.querySelectorAll(".dm-btn")).find(
        btn => btn.textContent.trim().startsWith(dmUsername)
    );

    if (!existingDM) {
        const newDM = document.createElement("button");
        newDM.className = "dm-btn";
        newDM.textContent = dmUsername;
        newDM.addEventListener("click", () => switchToDM(dmUsername));
        dmPane.appendChild(newDM);

        const dmList = JSON.parse(localStorage.getItem("dmList") || "[]");
        if (!dmList.includes(dmUsername)) {
            dmList.push(dmUsername);
            localStorage.setItem("dmList", JSON.stringify(dmList));
        }
    } else {
        if (!existingDM.querySelector(".notification-dot")) {
            const dot = document.createElement("span");
            dot.className = "notification-dot";
            dot.style.float = "right";
            dot.style.marginLeft = "auto";
            existingDM.appendChild(dot);
        }
    }
}

function restoreDMs() {
    const dmList = JSON.parse(localStorage.getItem("dmList") || "[]");
    dmList.forEach(username => createDM(username));
}
// === Server button resets to public chat ===
document.querySelector('.server-btn:first-child').addEventListener('click', () => {
    switchToDM(null);
});

//typing focuses on messagebar
document.addEventListener("keydown", function (event) {
    const input = document.getElementById("ClientMessage");

    // Ignore if already focused
    if (document.activeElement === input) return;
    if (event.key.length === 1) {
        input.focus();
        input.value += event.key;  
        event.preventDefault();    
    }
});

let lastUser=""
function displayMessage(text) {
    let data;
    try {
        data = JSON.parse(text);
    } catch {
        return;
    }

    const [user, message, type, to] = data;
    const isDM = type === "dm";
    const isSelf = user === username;

    if (isDM) {
        if (dm_recipient !== user && dm_recipient !== to) {
            console.log(`Skipping DM message not in current view: ${user} -> ${to}`);
            createDM(user) 
            return;
        }
    } else if (msg_type === "dm") {
        console.log(`Skipping public message while in DM view`);
        return;
    }

    //Only show the message if it matches current view
    if (isDM) {
        if (dm_recipient !== user && dm_recipient !== to) {
            return;
        }
    } else if (msg_type === "dm") {
        return;
    }

    const messagesDiv = document.getElementById("ChatBox");
    const messageContainer = document.createElement("div");

    if (user !== lastUser) {
        const messageUser = document.createElement("button");
        messageUser.textContent = user;
        messageContainer.appendChild(messageUser);
        lastUser = user;
    }

    const messageContent = document.createElement("p");
    messageContent.innerHTML = message
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/\n/g, "<br>");
    messageContainer.appendChild(messageContent);

    messageContainer.classList.add(
        user === "SERVER" ? "ServerMessage" :
        isSelf ? "SelfMessage" :
        "OtherMessage"
    );

    messagesDiv.appendChild(messageContainer);

    const shouldScroll = messagesDiv.scrollTop + messagesDiv.clientHeight >= messagesDiv.scrollHeight - 100;
    if (shouldScroll || isSelf) {
        requestAnimationFrame(() => {
            const messagesDiv = document.getElementById("ChatBox");
            messagesDiv.scrollTop = messagesDiv.scrollHeight;
        });
    }
}

async function insertRandomEmoji() {
    const res = await fetch("https://unpkg.com/emoji.json/emoji.json");
    const emoji = (await res.json())[Math.floor(Math.random() * 1000)].char;
    const input = document.getElementById("ClientMessage");
    input.setRangeText(emoji, input.selectionStart, input.selectionEnd, "end");
    input.focus();
    document.getElementById("emoji-button").textContent = emoji;
  }


const logoutButton = document.getElementById("logout-button");
if (logoutButton) {
  logoutButton.addEventListener("click", () => {
    document.cookie = "session=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;";
    window.location.href = "/auth.html"; // Redirect to login page
  });}