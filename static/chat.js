//let http_url="http://127.0.0.1:8080"
//let ws_url="ws://127.0.0.1:8080"
let http_url="https://rust-chat-um86.onrender.com"
let ws_url="wss://rust-chat-um86.onrender.com"


let socket
const statusElement = document.getElementById("themeToggle");
let username


//CHAT
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
    //statusElement.textContent = "Connected!";
    statusElement.style.color="#b8bb26";
    }
    else if(status==0){
        //statusElement.textContent = "Connecting...";
        statusElement.style.color="#83a598";
    }
    else if(status==-1){
        //statusElement.textContent = "Disconnected...(Refresh)";
        statusElement.style.color="#fb4934";
    }
}

function ConnectSocket() {
    ToggleStatus(0);
    socket = new WebSocket(`${ws_url}/chat`);
    socket.onopen = () => {
        ToggleStatus(1);
        let messagesDiv = document.getElementById("ChatBox");
        messagesDiv.scrollTop = messagesDiv.scrollHeight;
    };
    socket.onclose = () => {
        ToggleStatus(-1);
    };
    
    socket.onmessage = (event) =>{ console.log("Message received:", event.data);
    displayMessage(event.data);
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
    localStorage.setItem(`chat_${username || 'public'}`, JSON.stringify(messages));
}

function loadChat(username) {
    return JSON.parse(localStorage.getItem(`chat_${username || 'public'}`)) || [];
}

function renderChatMessages(messages) {
    const chatBox = document.getElementById('ChatBox');
    chatBox.innerHTML = '';
    messages.forEach(msg => {
        const p = document.createElement('p');
        p.textContent = msg;
        chatBox.appendChild(p);
    });
}

function appendMessage(username, message) {
    const messages = loadChat(username);
    messages.push(message);
    saveChat(username, messages);
    renderChatMessages(messages);
}

// === Switching logic ===
function switchToDM(username) {
    const chatTitle = document.getElementById('ChatTitle');
    const chatBox = document.getElementById('ChatBox');
    const messageInput = document.getElementById('ClientMessage');

    if (dm_recipient !== username) {
        msg_type = username ? "dm" : "broadcast";
        dm_recipient = username;

        chatTitle.textContent = username ? `DM: ${dm_recipient}` : 'Public Chat';
        renderChatMessages(loadChat(dm_recipient));
        messageInput.placeholder = username ? `Message ${dm_recipient}...` : `Type a message...`;
    }

    messageInput.focus();
}

// === Clicking on a chat user ===
document.getElementById("ChatBox").addEventListener("click", (event) => {
    if (event.target.tagName === "BUTTON") {
        let button_text = event.target.textContent.trim();        
        if (button_text.includes("::")) {
            button_text = button_text.split("::").pop().trim();
        }

        // Add user to DM list if not already present
        const dmPane = document.getElementById("dm-pane");
        const existingDM = Array.from(dmPane.querySelectorAll(".dm-btn")).find(
            btn => btn.textContent.trim() === button_text
        );

        if (!existingDM) {
            const newDM = document.createElement("button");
            newDM.className = "dm-btn";
            newDM.textContent = button_text;
            newDM.addEventListener("click", () => switchToDM(button_text));
            dmPane.appendChild(newDM);
        }

        switchToDM(button_text);
    }
});

// === Server button resets to public chat ===
document.querySelector('.server-btn:first-child').addEventListener('click', () => {
    switchToDM(null);
});




document.addEventListener("keydown", function (event) {
    if (event.key === "Escape") {
        msg_type = "broadcast";
        dm_recipient = null;
        const existingInfo = document.querySelector("#MessageBox h4");
        if (existingInfo) {
            existingInfo.remove();
        }
        document.getElementById("ClientMessage").focus();

    }
});

let lastUser=""
function displayMessage(text) {
    let user, message, messageContainer;
    try {
        [user, message] = JSON.parse(text);
    } catch {
        message = text;
    }

    let messagesDiv = document.getElementById("ChatBox");
    messageContainer = document.createElement("div");
    const nearBottom = messagesDiv.scrollHeight - messagesDiv.scrollTop <= messagesDiv.clientHeight + 40;

    if (user != lastUser) {
        const messageUser = document.createElement("button");
        messageUser.textContent = user;
        messageContainer.appendChild(messageUser);
        lastUser = user;
    }

    const messageContent = document.createElement("p");

    // Convert newlines to <br> and escape HTML
    messageContent.innerHTML = message
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/\n/g, "<br>");

    messageContainer.appendChild(messageContent);
    messagesDiv.appendChild(messageContainer);

    messageContainer.classList.add(
        user === "SERVER" ? "ServerMessage" : (user === username ? "SelfMessage" : "OtherMessage")
    );

    if (user === username || nearBottom) {
        messagesDiv.scrollTop = messagesDiv.scrollHeight;
    }
}
