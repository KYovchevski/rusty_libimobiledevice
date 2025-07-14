use aws_lc_rs::encoding::AsDer;
use rsa::{pkcs1::DecodeRsaPublicKey, pkcs8::DecodePublicKey};
use rustls_pki_types::pem::PemObject;
use spki::{
    der::{Decode, Encode},
    EncodePublicKey,
};
use std::{
    ffi::CStr,
    io::Write,
    net::{SocketAddr, TcpStream},
    os::windows::io::FromRawSocket,
    pin::Pin,
    str::FromStr,
    sync::Arc,
};
use x509_parser::prelude::FromDer;

use crate::bindings::{
    self, idevice_connection_type_CONNECTION_USBMUXD, idevice_error_t,
    idevice_error_t_IDEVICE_E_INVALID_ARG, idevice_error_t_IDEVICE_E_SSL_ERROR,
    idevice_error_t_IDEVICE_E_SUCCESS, key_data_t, pair_record_import_crt_with_name,
    pair_record_import_key_with_name, plist_t, userpref_error_t_USERPREF_E_SUCCESS,
    USERPREF_ROOT_CERTIFICATE_KEY, USERPREF_ROOT_PRIVATE_KEY_KEY,
};

type DeviceConnection = crate::bindings::idevice_connection_private;

struct ExternRustLsData {
    // rustls_connection: *mut rustls::ClientConnection,
    // tcp_stream: *mut TcpStream,

    // rustls_stream: Box<rustls::Stream<'a, rustls::ClientConnection, TcpStream>>,
    // rustls_stream: rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    rustls_stream: rustls::StreamOwned<rustls::ServerConnection, TcpStream>,
}

// This is defined in usbmuxd/usbmuxd-proto.h
// TODO: Get it in here somehow more reliably maybe?
#[cfg(target_os = "windows")]
const USBMUXD_SOCKET_PORT: u32 = 27015;

// unsafe fn connect_usbmuxd(connection: &DeviceConnection,
//     pair_record: plist_t) -> idevice_error_t {

//         idevice_error_t_IDEVICE_E_SUCCESS
//     }

// unsafe fn connect_network(connection: &DeviceConnection,
//     pair_record: plist_t) -> {

//     }

unsafe fn slice_from_key_data_t(key_data: &key_data_t) -> &[u8] {
    std::slice::from_raw_parts(key_data.data.cast(), key_data.size as usize)
}

#[no_mangle]
pub unsafe extern "C" fn extern_connection_enable_rustls(
    connection: *mut DeviceConnection,
    pair_record: plist_t,
) -> idevice_error_t {
    let Some(connection) = connection.as_mut() else {
        return idevice_error_t_IDEVICE_E_INVALID_ARG;
    };

    log::set_max_level(log::LevelFilter::Trace);

    // let ctx = &mut connection.data.cast::<ExternRustLsData>().read();

    // if device.conn_type == idevice_connection_type_CONNECTION_USBMUXD {
    //     connect_usbmuxd(connection, pair_record)
    // } else if {

    // }

    let mut root_cert = key_data_t {
        data: std::ptr::null_mut(),
        size: 0,
    };

    pair_record_import_crt_with_name(
        pair_record,
        USERPREF_ROOT_CERTIFICATE_KEY.as_ptr() as *const i8,
        &mut root_cert,
    );

    let cert_cstr = CStr::from_ptr(root_cert.data.cast());
    dbg!(cert_cstr);
    dbg!(cert_cstr.to_str().unwrap().len());
    let Ok(root_cert_der) =
        rustls_pki_types::CertificateDer::from_pem_slice(slice_from_key_data_t(&root_cert))
    else {
        return idevice_error_t_IDEVICE_E_SSL_ERROR;
    };

    libc::free(root_cert.data.cast());

    let mut root_private_key = key_data_t {
        data: std::ptr::null_mut(),
        size: 0,
    };

    pair_record_import_key_with_name(
        pair_record,
        USERPREF_ROOT_PRIVATE_KEY_KEY.as_ptr() as *const i8,
        &mut root_private_key,
    );

    let Ok(private_key_der) =
        rustls_pki_types::PrivateKeyDer::from_pem_slice(slice_from_key_data_t(&root_private_key))
    else {
        return idevice_error_t_IDEVICE_E_SSL_ERROR;
    };
    libc::free(root_private_key.data.cast());

    let mut root_store = rustls::RootCertStore::empty();
    root_store.add(root_cert_der.clone()).unwrap();

    // let config = rustls::ClientConfig::builder()
    //     .with_root_certificates(root_store)
    //     .with_no_client_auth();

    // let config = rustls::server::ServerConfig::builder()
    //     .with_no_client_auth()
    //     .with_single_cert(vec![root_cert_der], private_key_der)
    //     .unwrap();

    // let config_rc = Arc::new(config);
    let device = connection.device.read();

    // let (server_name, tcp_stream) =
    let mut tcp_stream = if device.conn_type == idevice_connection_type_CONNECTION_USBMUXD {
        let usbmuxd_port =
            std::env::var("USBMUXD_SOCKET_ADDRESS").unwrap_or(USBMUXD_SOCKET_PORT.to_string());

        let addr = format!("127.0.0.1:{usbmuxd_port}");
        let usbmuxd_addr = SocketAddr::from_str(addr.as_str()).unwrap();
        // let server_name = rustls_pki_types::ServerName::try_from(addr).unwrap();

        let fd = connection.data as u64;
        let mut tcp_stream = std::net::TcpStream::from_raw_socket(fd);
        // let fd = std::fs::File::from_raw_fd(fd);
        // let s = RawSocket::from(fd);

        // (server_name, tcp_stream)
        tcp_stream
    } else {
        todo!();

        let server_name = {
            let device = connection.device.read();
            let connection_address = device.conn_data.cast::<i8>().cast_const();
            let connection_address = std::ffi::CStr::from_ptr(connection_address)
                .to_str()
                .unwrap();
            rustls_pki_types::ServerName::try_from(connection_address).unwrap()
        };
    };

    tcp_stream.set_nonblocking(false).unwrap();

    // let tcp_stream = Box::new(tcp_stream);
    // let rustls_connection =
    // Box::new(rustls_connector::rustls::ClientConnection::new(config_rc.clone(), server_name).unwrap());
    // Box::new(rustls::ServerConnection::new(config_rc.clone()).unwrap());

    // let tcp_stream_ptr = Box::into_raw(tcp_stream);
    // let rustls_connection_ptr = Box::into_raw(rustls_connection);

    // let tcp_stream: &'static mut _ = tcp_stream_ptr.as_mut().unwrap();
    // let rustls_connection: &'static mut _ = rustls_connection_ptr.as_mut().unwrap();
    // let rustls_connector = rustls_connector::RustlsConnectorConfig::default()
    //     //new_with_native_certs().unwrap()
    //     // .connector_with_no_client_auth();
    //     .connector_with_single_cert(vec![root_cert_der], private_key_der)
    //     .unwrap();

    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![root_cert_der.clone()], private_key_der.clone_key())
        .unwrap();
    let server_config = Arc::new(server_config);
    let mut server_connection = rustls::ServerConnection::new(server_config).unwrap();

    #[derive(Debug)]
    struct ServerCertVerifier {
        root_cert_store: rustls::RootCertStore,
    };

    impl rustls::client::danger::ServerCertVerifier for ServerCertVerifier {
        fn verify_server_cert(
            &self,
            end_entity: &rustls_pki_types::CertificateDer<'_>,
            intermediates: &[rustls_pki_types::CertificateDer<'_>],
            server_name: &rustls_pki_types::ServerName<'_>,
            ocsp_response: &[u8],
            now: rustls_pki_types::UnixTime,
        ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
            use x509_parser::x509;

            assert!(intermediates.is_empty());

            let signature_verification_algorithms =
                rustls::crypto::aws_lc_rs::default_provider().signature_verification_algorithms;

            let cert = rustls::server::ParsedCertificate::try_from(end_entity)
                .expect("Failed to parse final certificate in server certificate chain.");

            let signature_res = rustls::client::verify_server_cert_signed_by_trust_anchor(
                &cert,
                &self.root_cert_store,
                intermediates,
                now,
                signature_verification_algorithms.all,
            );
            let name_res = rustls::client::verify_server_name(&cert, server_name);

            let cert = {
                let (rem, cert) = x509_parser::parse_x509_certificate(&end_entity)
                    .expect("Failed to parse x509 certificate");

                if rem.len() != 0 {
                    return Err(rustls::Error::InvalidCertificate(
                        rustls::CertificateError::BadEncoding,
                    ));
                }

                cert
            };

            match signature_res {
                Ok(_) => {}
                Err(rustls::Error::InvalidCertificate(
                    rustls::CertificateError::UnsupportedSignatureAlgorithm,
                )) => {
                    if cert.signature_algorithm.algorithm
                        != x509_parser::oid_registry::OID_PKCS1_SHA1WITHRSA
                    {
                        use x509_verify::{der, spki};

                        let alg = {
                            let alg = cert.signature_algorithm;

                            let parameters = alg.parameters.as_ref().map(|param| -> Result<der::Any, rustls::Error> {
                                let tag = der::Tag::try_from(param.tag().0 as u8).map_err(|_e| rustls::Error::InvalidCertificate(rustls::CertificateError::UnsupportedSignatureAlgorithm))?;
                                der::Any::new(tag, param.data).map_err(|_e| rustls::Error::InvalidCertificate(rustls::CertificateError::UnsupportedSignatureAlgorithm))
                            }).transpose()?;

                            spki::AlgorithmIdentifierOwned {
                                oid: spki::ObjectIdentifier::from_bytes(alg.oid().as_bytes())
                                    .unwrap(),
                                parameters,
                            }
                        };

                        let verify_info = x509_verify::VerifyInfo::new(
                            cert.tbs_certificate.as_ref().into(),
                            x509_verify::Signature::new(&alg, cert.signature_value.data),
                        );

                        let public_key = spki::SubjectPublicKeyInfo::from_der(
                            cert.tbs_certificate.subject_pki.raw,
                        )
                        .unwrap();
                        let key = x509_verify::VerifyingKey::new(public_key).unwrap();

                        if let Err(e) = key.verify(verify_info) {
                            dbg!(e);
                            return Err(rustls::Error::InvalidCertificate(
                                rustls::CertificateError::Other(rustls::OtherError(Arc::new(e))),
                            ));
                        }
                    }
                }
                Err(err) => return Err(err),
            };

            Ok(rustls::client::danger::ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            message: &[u8],
            cert: &rustls_pki_types::CertificateDer<'_>,
            dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            {
                let cert = x509_cert::Certificate::from_der(&cert).unwrap();

                let verify_info = x509_verify::VerifyInfo::new(
                    message.into(),
                    x509_verify::Signature::new(
                        &cert.signature_algorithm,
                        cert.signature.as_bytes().unwrap(),
                    ),
                );

                let key: x509_verify::VerifyingKey = cert
                    .tbs_certificate
                    .subject_public_key_info
                    .try_into()
                    .unwrap();

                key.verify(verify_info).unwrap();
            }
            todo!()
        }

        fn verify_tls13_signature(
            &self,
            message: &[u8],
            cert: &rustls_pki_types::CertificateDer<'_>,
            dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            todo!()
        }

        fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
            let signature_verification_algorithms =
                rustls::crypto::aws_lc_rs::default_provider().signature_verification_algorithms;
            let mut schemes = signature_verification_algorithms.supported_schemes();
            // The phone, which acts as the server in the TLS connection, can provide an RSA_PKCS1_SHA1 signed certificate.
            // When that happens, we will handle the certificate manually instead of relying on rustls
            // which explicitly doesn't support SHA1 signatures for security reasons.
            // Now, the phone doesn't seem deterred by the fact that we normally don't support SHA1 signatures, as normally
            // in the case that no supported certificate can be provided by the server, the handshake must fail.
            // However, we say that we will support RSA_PKCS1_SHA1 here for correctness.
            schemes.push(rustls::SignatureScheme::RSA_PKCS1_SHA1);

            schemes
        }
    }

    let mut client_connection = {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add(root_cert_der.clone());

        // let webpki_verifier = rustls::client::WebPkiServerVerifier::builder(roots).

        let client_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(ServerCertVerifier {
                root_cert_store: root_store,
            }))
            // .with_client_auth_cert(vec![root_cert_der.clone()], private_key_der.clone())
            .with_no_client_auth();

        let client_config = Arc::new(client_config);
        rustls::ClientConnection::new(
            client_config,
            // Address shouldn't matter?
            // https://github.com/rustls/rustls/issues/1026
            rustls_pki_types::ServerName::from(
                std::net::Ipv4Addr::from_str("69.69.69.69").unwrap(),
            ),
        )
        .unwrap()
    };

    // server_connection.

    {
        // let mut io = vec![0u8; 1024 * 256]; // 256KiB for IO?

        struct ScuffedIo<'a> {
            root_cert: rustls_pki_types::CertificateDer<'a>,
            tcp_stream: &'a mut TcpStream,
        };

        impl std::io::Read for ScuffedIo<'_> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                println!("Reading {} bytes", buf.len());
                let res = self.tcp_stream.read(buf);
                if let Ok(l) = res {
                    println!("Read {:?}", &buf[..l]);
                }

                res

                // let sl = &mut buf[..self.root_cert.len()];

                // sl.copy_from_slice(&self.root_cert.as_ref());

                // // println!("Read {}", str::from_utf8(buf).unwrap());
                // Ok(sl.len())
            }
        }

        impl std::io::Write for ScuffedIo<'_> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                println!("Writing {} bytes", buf.len());
                println!("Written {:?}", buf);

                self.tcp_stream.write(buf)

                // Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                self.tcp_stream.flush()
                // println!("Flushing");
                // Ok(())
            }
        }

        {
            let mut io = ScuffedIo {
                root_cert: root_cert_der.clone(),
                tcp_stream: &mut tcp_stream,
            };

            client_connection.complete_io(&mut io).unwrap();
        }
        while server_connection.is_handshaking() {
            if server_connection.wants_write() {
                server_connection.write_tls(&mut tcp_stream).unwrap();
                panic!();
            }
            while server_connection.wants_read() {
                let res = server_connection.read_tls(&mut tcp_stream).unwrap();
                if res != 0 {
                    println!("Read something {}", res);
                }
                let state = server_connection.process_new_packets().unwrap();
                if res == 0 {
                    break;
                }
            }

            // panic!();
        }
        panic!("Handshake done!");

        dbg!(server_connection.server_name());
        panic!();
        dbg!(server_connection.is_handshaking());
    }

    let rustls_stream = rustls::StreamOwned::new(server_connection, tcp_stream);
    // rustls_stream.
    // let rustls_stream = rustls_connector.connect("127.0.0.1", tcp_stream).unwrap();

    // let rustls_stream = Box::new(rustls::Stream::new(rustls_connection, tcp_stream));

    let external_data = Box::new(ExternRustLsData {
        // rustls_connection: rustls_connection_ptr,
        // tcp_stream: tcp_stream_ptr,
        rustls_stream,
    });

    let external_data = Box::into_pin(external_data);

    let ptr = Pin::into_inner_unchecked(external_data);
    connection.ssl_data = Box::into_raw(ptr).cast();

    idevice_error_t_IDEVICE_E_SUCCESS
}

/// Returns -1 on error and logs the error. If successful, returns the number of bytes sent instead.
#[no_mangle]
pub unsafe extern "C" fn extern_connection_rustls_send(
    connection: *mut DeviceConnection,
    data: *const std::ffi::c_void,
    data_len: bindings::size_t,
) -> bindings::ssize_t {
    let Some(connection) = connection.as_mut() else {
        panic!("Connection was a nullptr!");
    };

    let Some(rustls_data) = connection.ssl_data.cast::<ExternRustLsData>().as_mut() else {
        log::error!("RustLs Context was missing!");
        connection.status = idevice_error_t_IDEVICE_E_SSL_ERROR;
        return -1;
    };

    {
        let connection = &rustls_data.rustls_stream.conn;
        // std::thread::sleep(std::time::Duration::from_secs(15));
        dbg!(connection.protocol_version());
        // assert!(!connection.is_handshaking());
    }

    let data = std::slice::from_raw_parts(data.cast(), data_len as usize);

    match rustls_data.rustls_stream.write(data) {
        Ok(sent_len) => sent_len as bindings::ssize_t,
        Err(err) => {
            log::error!("Failed to write to RustLs Stream. Error: {err}");
            -1
        }
    }

    // panic!("extern_connection_rustls_send called");
}

/// Returns -1 on error and logs the error. If successful, returns the number of bytes sent instead.
#[no_mangle]
pub unsafe extern "C" fn extern_connection_rustls_recv(
    connection: *mut DeviceConnection,
    data: *mut std::ffi::c_void,
    data_len: bindings::size_t,
) -> bindings::ssize_t {
    panic!("Pls crash");
    let Some(connection) = connection.as_mut() else {
        panic!("Connection was a nullptr!");
    };

    let Some(rustls_data) = connection.ssl_data.cast::<ExternRustLsData>().as_mut() else {
        log::error!("RustLs Context was missing!");
        connection.status = idevice_error_t_IDEVICE_E_SSL_ERROR;
        return -1;
    };

    let data = std::slice::from_raw_parts(data.cast(), data_len as usize);

    match rustls_data.rustls_stream.write(data) {
        Ok(sent_len) => sent_len as bindings::ssize_t,
        Err(err) => {
            log::error!("Failed to write to RustLs Stream. Error: {err}");
            -1
        }
    }

    // panic!("extern_connection_rustls_send called");
}

#[no_mangle]
pub unsafe extern "C" fn extern_generate_keys_and_certs(
    pair_record: plist_t,
    public_key: key_data_t,
    dev_cert_pem: &mut key_data_t,
    root_key_pem: &mut key_data_t,
    root_cert_pem: &mut key_data_t,
    host_key_pem: &mut key_data_t,
    host_cert_pem: &mut key_data_t,
) -> bindings::userpref_error_t {
    // use aws_lc_rs as aws;
    use x509_parser as x509;

    let public_key = slice_from_key_data_t(&public_key);
    let public_key_pem = str::from_utf8(public_key).unwrap();
    // dbg!(str::from_utf8(public_key));
    let (_, pem) = x509::pem::parse_x509_pem(public_key).expect("Failed to parse public key PEM");

    let device_public_key = {
        // Respectfully, what? This doesn't fully make sense to me, or at least the solution seems wack.
        // We get an RSA public key for the device, which we need to first parse as PEM through `rsa`.
        let key = rsa::RsaPublicKey::from_pkcs1_pem(&public_key_pem)
            .expect("Failed to parse device public key");
        // Then we convert this back to DER agaisnt using `impl spki::EncodePublicKey for rsa::PublicKey`.
        let spki_der = key.to_public_key_der().unwrap();
        // ONLY THEN rcgen::SubjectPublicKeyInfo::from_der() doesn't fail.
        rcgen::SubjectPublicKeyInfo::from_der(spki_der.as_bytes()).unwrap()

        // As far as I understand this is caused by there being a significant difference between a PEM `PUBLIC KEY` and a PEM `RSA PUBLIC KEY`,
        // and that difference is enough to break rcgen. Namely, an `RSA PUBLIC KEY`` is only an exponent and modulus, while a `PUBLIC KEY` contains a signature algorithm which `rcgen` expects.
        // This would mean that `rsa` picks out an algorithm, which from the source code seems to be `pkcs1`.
        // However, even if that is the case and I didn't just randomly stumble into a solution, this solution seems strange.
    };
    // let device_public_key = dbg!(rcgen::SubjectPublicKeyInfo::from_pem(dbg!(&str::from_utf8(public_key).unwrap()))).unwrap();

    // rcgen uses ECDSA and as far as I can tell cannot handle RSA public keys at all, which is what we are getting from libusbmuxd,
    // which I presume is a key received from the device.
    // Doesn't seem to have any significant implications for our usecase besides changing what crate is used.

    let root_keypair = rcgen::KeyPair::generate().expect("Failed to generate root keypair");
    let host_keypair = rcgen::KeyPair::generate().expect("Failed to generate host keypair");

    // TODO: Valid dates for the certificates. The default is 31 Dec 1974 to 31 Dec 4095. Bit too long.

    let root_cert = {
        let mut cert_params =
            rcgen::CertificateParams::new(["rusty_libimobiledevice".into()]).unwrap();
        cert_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        cert_params.distinguished_name.push(
            rcgen::DnType::CommonName,
            "rusty_libimobiledevice root cert",
        );
        cert_params
            .self_signed(&root_keypair)
            .expect("Failed to generate root cert")
    };

    let host_cert = {
        let mut cert_params =
            rcgen::CertificateParams::new(["rusty_libimobiledevice".into()]).unwrap();
        cert_params.is_ca = rcgen::IsCa::ExplicitNoCa;
        cert_params.distinguished_name.push(
            rcgen::DnType::CommonName,
            "rusty_libimobiledevice host cert",
        );
        cert_params
            .key_usages
            .push(rcgen::KeyUsagePurpose::DigitalSignature);
        cert_params
            .key_usages
            .push(rcgen::KeyUsagePurpose::KeyEncipherment);

        cert_params
            .signed_by(&host_keypair, &root_cert, &root_keypair)
            .expect("Failed to generate host cert")
    };

    let device_cert = {
        let mut cert_params =
            rcgen::CertificateParams::new(["rusty_libimobiledevice".into()]).unwrap();
        cert_params.is_ca = rcgen::IsCa::ExplicitNoCa;
        cert_params.distinguished_name.push(
            rcgen::DnType::CommonName,
            "rusty_libimobiledevice device cert",
        );
        // cert_params.key_identifier_method
        cert_params
            .key_usages
            .push(rcgen::KeyUsagePurpose::DigitalSignature);
        cert_params
            .key_usages
            .push(rcgen::KeyUsagePurpose::KeyEncipherment);

        {
            // https://docs.redhat.com/en/documentation/red_hat_certificate_system/9/html/administration_guide/standard_x.509_v3_certificate_extensions#Standard_X.509_v3_Certificate_Extensions-subjectKeyIdentifier
            let oid = x509::oid_registry::OID_X509_EXT_SUBJECT_KEY_IDENTIFIER
                .iter()
                .expect("OID_X509_EXT_SUBJECT_KEY_IDENTIFIER could not be converted to Vec<u64>")
                .collect::<Vec<_>>();
            let ski_extension =
                rcgen::CustomExtension::from_oid_content(&oid, "hash".as_bytes().to_vec());
            cert_params.custom_extensions.push(ski_extension);
        }

        cert_params
            .signed_by(&device_public_key, &root_cert, &root_keypair)
            .expect("Failed to generate device cert")
    };

    unsafe fn pem_to_key_data_t(pem: String) -> key_data_t {
        let pem = pem.into_boxed_str();
        let pem_len = pem.len();
        let pem_str = Box::into_raw(pem);
        let pem_bytes = pem_str.as_mut().unwrap().as_mut_ptr();

        key_data_t {
            data: pem_bytes,
            size: pem_len as u32,
        }
    }

    *dev_cert_pem = pem_to_key_data_t(device_cert.pem());
    *root_cert_pem = pem_to_key_data_t(root_cert.pem());
    *host_cert_pem = pem_to_key_data_t(host_cert.pem());

    *root_key_pem = pem_to_key_data_t(root_keypair.serialize_pem());
    *host_key_pem = pem_to_key_data_t(host_keypair.serialize_pem());

    dbg!(CStr::from_ptr(root_cert_pem.data.cast()));

    // panic!("Cerfiticates successfully generated");
    userpref_error_t_USERPREF_E_SUCCESS
}
