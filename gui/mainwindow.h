#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include <QWebSocket>  // Add this line

QT_BEGIN_NAMESPACE
namespace Ui {
class MainWindow;
}
QT_END_NAMESPACE

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    MainWindow(QWidget *parent = nullptr);
    ~MainWindow();

private slots:
    void on_connectButton_clicked();
    void onConnected();
    void on_messagetxt_returnPressed();
    void onTextMessageReceived(QString message);

private:
    Ui::MainWindow *ui;
    QWebSocket *socket;
};

#endif // MAINWINDOW_H
