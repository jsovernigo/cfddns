FROM alpine

RUN mkdir /etc/cfddns
WORKDIR /etc/cfddns

ADD ./cfddns.sh /etc/cfddns/

ENV APIBASE="https://api.cloudflare.com/client/v4"

RUN apk add --update bind-tools curl jq

ENTRYPOINT [ "/etc/cfddns/cfddns.sh" ]
