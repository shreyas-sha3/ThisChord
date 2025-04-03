const socket = new WebSocket("ws://localhost:8080/chat");

socket.onopen = () => console.log("Connected to WebSocket");
socket.onmessage = (event) =>{ console.log("Message received:", event.data);

displayMessage(event.data);
}

function sendMessage(event){
    event.preventDefault();
    let MessageBox = document.getElementById("ClientMessage");
    socket.send(MessageBox.value);
    displayMessage(MessageBox.value);
    MessageBox.value = "";
}
lastUser=""
function displayMessage(text) {
        let user,message,messageContainer;

        try {
            [user, message] = JSON.parse(text);
        } catch {
            message = text;
        }
        
        let messagesDiv = document.getElementById("ChatBox");

        if(user==lastUser){
            messageContainer = messagesDiv.lastElementChild;
        }
        else{
            messageContainer = document.createElement("div");
            messageUser = document.createElement("h4");
            messageUser.textContent = user || "You";
            messageContainer.appendChild(messageUser);
            lastUser=user;
        }

        let messageContent = document.createElement("p");
        messageContent.textContent = message;        
        messageContainer.appendChild(messageContent);
        messagesDiv.appendChild(messageContainer);

        messageContainer.classList.add(
            user === "SERVER" ? "ServerMessage" : (user ? "OtherMessage" : "SelfMessage")
        );        
        messagesDiv.scrollTop = messagesDiv.scrollHeight;

    }
