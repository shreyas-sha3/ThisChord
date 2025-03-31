#include "mainwindow.h"
#include "./ui_mainwindow.h"
#include <QDebug>

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , ui(new Ui::MainWindow)
    , socket(nullptr)  // Initialize as nullptr
{
    ui->setupUi(this);
    ui->chatBox->setReadOnly(true);

}

MainWindow::~MainWindow()
{
    delete ui;
    if (socket) {
        socket->deleteLater();  // Ensures cleanup
    }
}

// Function to handle WebSocket connection on button press
void MainWindow::on_connectButton_clicked()
{
    if (!socket) {
        socket = new QWebSocket();
        connect(socket, &QWebSocket::connected, this, []() {
            qDebug() << "Connected!";
        });
        connect(socket, &QWebSocket::textMessageReceived, this, &MainWindow::onTextMessageReceived);
        socket->open(QUrl("wss://rust-chat-um86.onrender.com/chat"));
        ui->connectButton->setText(QString("Disconnect"));
    }
    else{
        socket->sendTextMessage(QString(":q"));
        socket->close();
        socket=nullptr;
        ui->connectButton->setText(QString("Connect"));
    }
}

// Called when WebSocket successfully connects
void MainWindow::onConnected()
{
    qDebug() << "WebSocket Connected!";
}

// Handles sending messages when Enter is pressed in the message box
void MainWindow::on_messagetxt_returnPressed()
{
    QString message = ui->messagetxt->text();
    qDebug() << "User entered: " << message;

    if (socket && socket->isValid()) {
        socket->sendTextMessage(message);
    } else {
        qDebug() << "WebSocket is not connected!";
    }

    ui->messagetxt->clear();
}

// Handles incoming WebSocket messages
void MainWindow::onTextMessageReceived(QString message)
{
    qDebug() << "Message received:" << message;
    ui->chatBox->append(message);
}
