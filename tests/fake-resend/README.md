# Fake Resend (MAIR-522)

The compose stacks run [WireMock](https://wiremock.org/) in place of the Resend API: Core is
started with `RESEND_BASE_URL=http://fake-resend:8080`, so `POST /emails` lands here, is
answered `200` with a random id (`mappings/emails.json`) and kept in the request journal.

Read the e-mails Core sent (template alias, variables, sender) from the host of the dev stack:

```bash
curl -s http://localhost:8025/__admin/requests | jq '.requests[].request.body | fromjson'
```

`load-test.js` reads the reset token the same way, through `POST /__admin/requests/find`.
To send real e-mails from the dev stack instead, start it with
`RESEND_API_KEY=re_… RESEND_BASE_URL=https://api.resend.com EMAIL_DOMAIN=mairie360-eip.fr`.
