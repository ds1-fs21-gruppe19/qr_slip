from qr_generator import create_qr_code

data = {
    "creditor_iban": "CH4000777003656120095",
    "creditor_name": "Tobias Rothlin",
    "creditor_address": "Peterliwiese 33",
    "creditor_zip_code": "8855",
    "creditor_city": "Wangen SZ",
    "creditor_country": "CH",
    "debtor_name": "Hans Muster",
    "debtor_address": "Sonnenstrasse 31",
    "debtor_zip_code": "2000",
    "debtor_city": "Schöningen",
    "debtor_country": "CH",
    "amount": "5000.00",
    "currency": "CHF",
    "reference_type": "SCOR",
    "reference_number": "test123",
    "additional_information": "Test123",
}

fileAsString = create_qr_code(data)
f = open("newQrCode.svg", "w")
f.write(fileAsString)
f.close()
