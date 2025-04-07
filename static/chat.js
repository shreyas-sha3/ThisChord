//let url="://127.0.0.1:8080"
let url="://rust-chat-um86.onrender.com"


let socket
const statusElement = document.getElementById("connection-status");
let username


//CHAT
fetch(`https${url}/auth-check`, {
    credentials: "include"
  })
  .then(res => res.json())
  .then(data => {
    if (data.status === "ok") {
      username = data.username;
    } else {
    window.location.href = "./auth.html";
    }
  });

ConnectSocket()

//CHAT

function ToggleStatus(status){
    if(status==1){
    statusElement.textContent = "Connected!";
    statusElement.style.color="#b8bb26";
    }
    else if(status==0){
        statusElement.textContent = "Connecting...";
        statusElement.style.color="#83a598";
    }
    else if(status==-1){
        statusElement.textContent = "Disconnected...(Refresh)";
        statusElement.style.color="#fb4934";
    }
}

function ConnectSocket() {
    ToggleStatus(0);
    socket = new WebSocket(`wss${url}/chat`);
    socket.onopen = () => {
        ToggleStatus(1);
    };
}

socket.onclose = () => {
    ToggleStatus(-1);
};

socket.onmessage = (event) =>{ console.log("Message received:", event.data);
displayMessage(event.data);
}


function sendMessage(event){
    event.preventDefault();
    let MessageBox = document.getElementById("ClientMessage");
    msg=MessageBox.value
    if(msg.trim()){
    socket.send(msg);
    displayMessage(JSON.stringify([username,msg]));;
    MessageBox.value = "";
}}
lastUser=""
function displayMessage(text) {
        let user,message,messageContainer;
        try {
            [user, message] = JSON.parse(text);
        } catch {
            message = text;
        }
        
        let messagesDiv = document.getElementById("ChatBox");
        messageContainer = document.createElement("div");
        const nearBottom = messagesDiv.scrollHeight - messagesDiv.scrollTop <= messagesDiv.clientHeight + 20;

        if(user!=lastUser){
            messageUser = document.createElement("h4");
            messageUser.textContent = user;
            messageContainer.appendChild(messageUser);
            lastUser=user;
        }

        let messageContent = document.createElement("p");
        messageContent.textContent = message;        
        messageContainer.appendChild(messageContent);
        messagesDiv.appendChild(messageContainer);

        messageContainer.classList.add(
            user === "SERVER" ? "ServerMessage" : (user===username ? "SelfMessage" : "OtherMessage")
        );
        if (user === username || nearBottom) {        
        messagesDiv.scrollTop = messagesDiv.scrollHeight;
        }
    }

const logoutButton = document.getElementById("logout-button");
if (logoutButton) {
  logoutButton.addEventListener("click", () => {
    document.cookie = "session=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;";
    window.location.href = "/auth.html"; // Redirect to login page
  });
}