import io

import qrcode
import qrcode.image.svg


def create_qr_code(json):
    qr_data = "SPC\n" \
              "0200\n" \
              "1\n" \
              + json["creditor_iban"] \
              + "\nK\n" \
              + json["creditor_name"] + "\n" \
              + json["creditor_address"] + "\n" \
              + json["creditor_zip_code"] + " " + json["creditor_city"] + "\n" \
              + "\n\n" \
              + json["creditor_country"] \
              + "\n\n\n\n\n\n\n\n" \
              + json["amount"] + "\n" \
              + json["currency"] + "\n" \
              + "K\n" \
              + json["debtor_name"] + "\n" \
              + json["debtor_address"] + "\n" \
              + json["debtor_zip_code"] + " " + json["debtor_city"] + "\n" \
              + "\n\n" \
              + json["debtor_country"] + "\n" \
              + json["reference_type"] + "\n" \
              + json["reference_number"] + "\n" \
              + json["additional_information"] + "\n" \
              + "EPD"

    img = qrcode.make(qr_data, image_factory=qrcode.image.svg.SvgImage)
    buffered = io.BytesIO()
    img.save(buffered, "SVG")

    xml_str = buffered.getvalue().decode("utf-8")

    closing_tag_pos = xml_str.rfind("</svg>")

    if closing_tag_pos >= 0:
        swiss_cross = """
        <rect x="25.9mm" y="25.9mm" class="st0" width="9.2mm" height="9.2mm"/>
        <rect x="27.5mm" y="27.5mm" width="6mm" height="6mm"/>
        <rect x="28.5mm" y="30mm" class="st0" width="4mm" height="1mm"/>
        <rect x="30mm" y="28.5mm" class="st0" width="1mm" height="4mm"/>
        <style type="text/css">.st0{fill:#FFFFFF;}</style>
        """

        xml_str = xml_str[:closing_tag_pos] + swiss_cross + xml_str[closing_tag_pos:]

    return xml_str
