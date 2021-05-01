import qrcode
import qrcode.image.svg
import io

def createQRCode(json):

    qrMessage = "SPC\n0200\n1\n"+json["Receiver_IBAN"].replace(' ', "")+"""\nS\n"""+\
                json["Receiver_Name"] + "\n" + json["Receiver_Street"].split(' ')[0] + \
                "\n"  + json["Receiver_Street"].split(' ')[1] + "\n" + \
                json["Receiver_City"][:4] + "\n" + json["Receiver_City"][4:].replace(' ', "") + \
                "\n" + "CH" + "\n\n\n\n\n\n\n\n" + json["Amount"] + "\n" + json["Currency"] + "\nS\n" + \
                json["FromName"] + "\n" + json["FromStreet"].split(' ')[0] + \
                "\n" + json["FromStreet"].split(' ')[1] + "\n" + \
                json["FromCity"][:4] + "\n" + json["FromCity"][4:].replace(' ',"") + \
                "\n" + "CH\nNON\n\n" + json["AdditionalInfo"] + "\nEPD"

    img = qrcode.make(qrMessage, image_factory = qrcode.image.svg.SvgImage)
    buffered = io.BytesIO()
    img.save(buffered, "SVG")

    splitXml = str(buffered.getvalue()).split(">")

    outputFile = """<?xml version='1.0' encoding='UTF-8'?>\n<svg width="61mm" height="61mm" version="1.1" xmlns="http://www.w3.org/2000/svg">"""

    for line in splitXml[2:-2]:
        outputFile += line + ">\n"

    outputFile += """<rect x="25.9mm" y="25.9mm" class="st0" width="9.2mm" height="9.2mm"/>\n<rect x="27.5mm" y="27.5mm" width="6mm" height="6mm"/>\n<rect x="28.5mm" y="30mm" class="st0" width="4mm" height="1mm"/>\n<rect x="30mm" y="28.5mm" class="st0" width="1mm" height="4mm"/>\n<style type="text/css">.st0{fill:#FFFFFF;}</style>"""
    outputFile += splitXml[-2] + ">"
    
    return outputFile



data = {
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
    "Currency": "CHF",
    }




fileAsString = createQRCode(data)
print(fileAsString)
f = open("./newQrCode.svg", "w")
f.write(fileAsString)
f.close()

