import qrcode
import qrcode.image.svg
from PIL import Image


def createQRCode(json, svgPath = "./QRCode.svg"):


    qrMessage = "SPC\n0200\n1\n" + json["InvoiceInfo"]["Receiver_IBAN"].replace(' ', "") + """\nS\n""" + \
                json["InvoiceInfo"]["Receiver_Name"] + "\n" + json["InvoiceInfo"]["Receiver_Street"].split(' ')[0] + \
                "\n"  + json["InvoiceInfo"]["Receiver_Street"].split(' ')[1] + "\n" + \
                json["InvoiceInfo"]["Receiver_City"][:4] + "\n" + json["InvoiceInfo"]["Receiver_City"][4:].replace(' ', "") + \
                "\n" + "CH" + "\n\n\n\n\n\n\n\n" + json["InvoiceInfo"]["Amount"] + "\nCHF\nS\n" + \
                json["InvoiceInfo"]["FromName"] + "\n" + json["InvoiceInfo"]["FromStreet"].split(' ')[0] + \
                "\n" + json["InvoiceInfo"]["FromStreet"].split(' ')[1] + "\n" + \
                json["InvoiceInfo"]["FromCity"][:4] + "\n" + json["InvoiceInfo"]["FromCity"][4:].replace(' ',"") + \
                "\n" + "CH\nNON\n\n" + json["InvoiceInfo"]["AdditionalInfo"] + "\nEPD"

    img = qrcode.make(qrMessage, image_factory = qrcode.image.svg.SvgImage)
    img.save(svgPath, "SVG")
    f = open(svgPath, "r")
    original = f.read()
    f.close()
    splitXml = original.split(">")
    outputFile = ""
    for line in splitXml[:-2]:
        outputFile += line + ">\n"
    outputFile += """<rect x="25.9mm" y="25.9mm" class="st0" width="9.2mm" height="9.2mm"/>\n<rect x="27.5mm" y="27.5mm" width="6mm" height="6mm"/>\n<rect x="28.5mm" y="30mm" class="st0" width="4mm" height="1mm"/>\n<rect x="30mm" y="28.5mm" class="st0" width="1mm" height="4mm"/>\n<style type="text/css">.st0{fill:#FFFFFF;}</style>"""
    outputFile += splitXml[-2] + ">"

    print(outputFile)

    f = open("./output.svg" , "w")
    f.write(outputFile)
    f.close()
    return svgPath


data = {
        "InvoiceInfo":
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
            "Amount": "5000.00"
        },
        "MetaData":
        {
            "NumberOfPages" : 1
        }
    }


print(createQRCode(data))

