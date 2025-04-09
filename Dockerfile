FROM python:3

RUN mkdir /etc/cfddns
WORKDIR /etc/cfddns

RUN pip install requests dotenv

ADD .env /etc/cfddns/

ADD ./cfddns.py /etc/cfddns/

ENTRYPOINT [ "python", "/etc/cfddns/cfddns.py" ]
