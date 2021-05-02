# QR Generator

### create_qr_code

The function `create_qr_code` takes a JSON data structure equal to the one expected by the `/generate-slip` endpoint and
encodes it into a QR code according to the [six documentation](https://www.paymentstandards.ch/dam/downloads/ig-qr-bill-de.pdf).

##### JSON structure
```json
{
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
```

The script qr_test.py may be used to test the QR code generation by saving the resulting SVG to `newQrCode.svg`.
