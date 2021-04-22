# QR Generator

### createQRCode

Die Funktion **createQRCode** erstellt aus der json Sturktur einen QRCode welcher dann als svg string zurück gegeben wird.
##### Funktions Signatur:
```
def createQRCode(json):
```
##### Paramerter:
- json -> json Struktur
##### json Struktur:
```
{
    "Receiver_IBAN" : "CH40 0077 7003 6561 2009 5",
    "Receiver_Name": "Tobias Rothlin",
    "Receiver_Street": "Peterliwiese 33",
    "Receiver_City": "8855 Wangen SZ",
    "Receiver_Ref": "",
    "AdditionalInfo": "Test123",
    "FromName": "Hans Muster",
    "FromStreet": "Sonnenstrasse 31",
    "FromCity": "2000 Schöningen",
    "Amount": "5000.00",
    "Currency": "CHF"
}
```
##### generierter QRCode:
![Invoice Modul](newQrCode.svg)

