--
-- PostgreSQL database dump
--

-- Dumped from database version 13.2 (Debian 13.2-1.pgdg100+1)
-- Dumped by pg_dump version 13.2

-- Started on 2021-04-14 10:14:42 UTC

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- TOC entry 207 (class 1255 OID 16391)
-- Name: diesel_manage_updated_at(regclass); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.diesel_manage_updated_at(_tbl regclass) RETURNS void
    LANGUAGE plpgsql
    AS $$
BEGIN
    EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s
                    FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at()', _tbl);
END;
$$;


ALTER FUNCTION public.diesel_manage_updated_at(_tbl regclass) OWNER TO postgres;

--
-- TOC entry 208 (class 1255 OID 16392)
-- Name: diesel_set_updated_at(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.diesel_set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.diesel_set_updated_at() OWNER TO postgres;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- TOC entry 200 (class 1259 OID 16385)
-- Name: __diesel_schema_migrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.__diesel_schema_migrations (
    version character varying(50) NOT NULL,
    run_on timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


ALTER TABLE public.__diesel_schema_migrations OWNER TO postgres;

--
-- TOC entry 202 (class 1259 OID 16395)
-- Name: principal; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.principal (
    pk integer NOT NULL,
    user_name character varying(255) NOT NULL,
    password character varying(255) NOT NULL
);


ALTER TABLE public.principal OWNER TO postgres;

--
-- TOC entry 201 (class 1259 OID 16393)
-- Name: principal_pk_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.principal_pk_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.principal_pk_seq OWNER TO postgres;

--
-- TOC entry 2983 (class 0 OID 0)
-- Dependencies: 201
-- Name: principal_pk_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.principal_pk_seq OWNED BY public.principal.pk;


--
-- TOC entry 204 (class 1259 OID 16408)
-- Name: qr_user; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.qr_user (
    pk integer NOT NULL,
    first_name character varying(255),
    last_name character varying(255),
    address character varying(255) NOT NULL,
    zip_code character varying(255) NOT NULL,
    city character varying(255) NOT NULL,
    iban character varying(255) NOT NULL,
    country character varying(255) NOT NULL,
    fk_principal integer NOT NULL
);


ALTER TABLE public.qr_user OWNER TO postgres;

--
-- TOC entry 203 (class 1259 OID 16406)
-- Name: qr_user_pk_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.qr_user_pk_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.qr_user_pk_seq OWNER TO postgres;

--
-- TOC entry 2984 (class 0 OID 0)
-- Dependencies: 203
-- Name: qr_user_pk_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.qr_user_pk_seq OWNED BY public.qr_user.pk;


--
-- TOC entry 206 (class 1259 OID 16424)
-- Name: refresh_token; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.refresh_token (
    pk integer NOT NULL,
    uuid uuid NOT NULL,
    expiry timestamp with time zone NOT NULL,
    invalidated boolean NOT NULL,
    fk_principal integer NOT NULL
);


ALTER TABLE public.refresh_token OWNER TO postgres;

--
-- TOC entry 205 (class 1259 OID 16422)
-- Name: refresh_token_pk_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.refresh_token_pk_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.refresh_token_pk_seq OWNER TO postgres;

--
-- TOC entry 2985 (class 0 OID 0)
-- Dependencies: 205
-- Name: refresh_token_pk_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.refresh_token_pk_seq OWNED BY public.refresh_token.pk;


--
-- TOC entry 2824 (class 2604 OID 16398)
-- Name: principal pk; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.principal ALTER COLUMN pk SET DEFAULT nextval('public.principal_pk_seq'::regclass);


--
-- TOC entry 2825 (class 2604 OID 16411)
-- Name: qr_user pk; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.qr_user ALTER COLUMN pk SET DEFAULT nextval('public.qr_user_pk_seq'::regclass);


--
-- TOC entry 2826 (class 2604 OID 16427)
-- Name: refresh_token pk; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.refresh_token ALTER COLUMN pk SET DEFAULT nextval('public.refresh_token_pk_seq'::regclass);

--
-- TOC entry 2986 (class 0 OID 0)
-- Dependencies: 201
-- Name: principal_pk_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.principal_pk_seq', 1, false);


--
-- TOC entry 2987 (class 0 OID 0)
-- Dependencies: 203
-- Name: qr_user_pk_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.qr_user_pk_seq', 1, false);


--
-- TOC entry 2988 (class 0 OID 0)
-- Dependencies: 205
-- Name: refresh_token_pk_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.refresh_token_pk_seq', 1, false);


--
-- TOC entry 2828 (class 2606 OID 16390)
-- Name: __diesel_schema_migrations __diesel_schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.__diesel_schema_migrations
    ADD CONSTRAINT __diesel_schema_migrations_pkey PRIMARY KEY (version);


--
-- TOC entry 2830 (class 2606 OID 16403)
-- Name: principal principal_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.principal
    ADD CONSTRAINT principal_pkey PRIMARY KEY (pk);


--
-- TOC entry 2832 (class 2606 OID 16405)
-- Name: principal principal_user_name_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.principal
    ADD CONSTRAINT principal_user_name_key UNIQUE (user_name);


--
-- TOC entry 2834 (class 2606 OID 16416)
-- Name: qr_user qr_user_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.qr_user
    ADD CONSTRAINT qr_user_pkey PRIMARY KEY (pk);


--
-- TOC entry 2836 (class 2606 OID 16429)
-- Name: refresh_token refresh_token_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.refresh_token
    ADD CONSTRAINT refresh_token_pkey PRIMARY KEY (pk);


--
-- TOC entry 2838 (class 2606 OID 16431)
-- Name: refresh_token refresh_token_uuid_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.refresh_token
    ADD CONSTRAINT refresh_token_uuid_key UNIQUE (uuid);


--
-- TOC entry 2839 (class 2606 OID 16417)
-- Name: qr_user qr_user_fk_principal_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.qr_user
    ADD CONSTRAINT qr_user_fk_principal_fkey FOREIGN KEY (fk_principal) REFERENCES public.principal(pk);


--
-- TOC entry 2840 (class 2606 OID 16432)
-- Name: refresh_token refresh_token_fk_principal_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.refresh_token
    ADD CONSTRAINT refresh_token_fk_principal_fkey FOREIGN KEY (fk_principal) REFERENCES public.principal(pk);


-- Completed on 2021-04-14 10:14:42 UTC

--
-- PostgreSQL database dump complete
--

